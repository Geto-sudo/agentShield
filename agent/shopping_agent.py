"""AgentShield demo: an AI shopping agent that has to ask before it pays.

The agent can search a small product catalog, but it has no way to pay on its
own. Its only path to money is the AgentShield API: every purchase is submitted
as an action, checked against the owner's mandate, and only an APPROVE (or an
ESCALATE confirmed by the owner) can turn into a PayPal Sandbox checkout.

Two brains are available:

* OpenAI (default when OPENAI_API_KEY is set): a real LLM drives the agent
  through function calling and decides which tool to use next.
* Offline planner (--offline, or no API key): a deterministic script that
  follows the same path, so the demo runs without any key.

Only the Python standard library is used.

Usage:
    python agent/shopping_agent.py "Find me wireless headphones and buy them"
    python agent/shopping_agent.py --offline --auto-confirm
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

CATALOG_PATH = Path(__file__).with_name("catalog.json")
DEFAULT_TASK = "Find me good wireless headphones and buy them."

SYSTEM_PROMPT = """You are Atlas Shopper, an autonomous shopping agent working for a human owner.

You can search products, but you cannot pay directly. Every purchase must go
through AgentShield, the owner's spending-control layer, by calling
request_purchase. AgentShield answers with one decision:

- APPROVE: you may call create_paypal_checkout with the returned action_id.
- ESCALATE: the owner must confirm. Call wait_for_owner_approval, then create
  the checkout only if the approval is confirmed.
- REFUSE: never try to pay for that product. Read the reasons. You may try
  another product that clearly fits the reasons (for example a cheaper one),
  but do not retry the same product and do not split a purchase to get around
  a limit.

Prefer the best-rated product first. Keep your final answer short: what you
bought (or why you could not), and the PayPal checkout link if there is one."""

TOOLS = [
    {
        "type": "function",
        "function": {
            "name": "search_products",
            "description": "Search the store catalog. Returns products with price, merchant, rating and delivery time.",
            "parameters": {
                "type": "object",
                "properties": {"query": {"type": "string", "description": "What to look for"}},
                "required": ["query"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "request_purchase",
            "description": "Submit a purchase to AgentShield for a decision. This is the only way to spend money.",
            "parameters": {
                "type": "object",
                "properties": {
                    "product_id": {"type": "string"},
                    "reason": {"type": "string", "description": "Why this product fits the owner's request"},
                },
                "required": ["product_id"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "wait_for_owner_approval",
            "description": "Wait for the owner to confirm or reject an escalated purchase.",
            "parameters": {
                "type": "object",
                "properties": {"approval_id": {"type": "string"}},
                "required": ["approval_id"],
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "create_paypal_checkout",
            "description": "Create the PayPal Sandbox checkout for an approved action.",
            "parameters": {
                "type": "object",
                "properties": {"action_id": {"type": "string"}},
                "required": ["action_id"],
            },
        },
    },
]


# --------------------------------------------------------------------------
# Small HTTP helper (standard library only)
# --------------------------------------------------------------------------


def http_json(method: str, url: str, body: dict | None = None, headers: dict | None = None, timeout: int = 30):
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(url, data=data, method=method)
    req.add_header("Content-Type", "application/json")
    for key, value in (headers or {}).items():
        req.add_header(key, value)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            return resp.status, json.loads(resp.read() or b"{}")
    except urllib.error.HTTPError as err:
        raw = err.read()
        try:
            return err.code, json.loads(raw or b"{}")
        except json.JSONDecodeError:
            return err.code, {"error": raw.decode(errors="replace")}


# --------------------------------------------------------------------------
# Output
# --------------------------------------------------------------------------

COLOR = sys.stdout.isatty() and not os.environ.get("NO_COLOR")


def paint(text: str, code: str) -> str:
    return f"\033[{code}m{text}\033[0m" if COLOR else text


def say(who: str, text: str, code: str = "0") -> None:
    print(f"{paint(f'[{who}]', code)} {text}", flush=True)


DECISION_COLOR = {"APPROVE": "32", "ESCALATE": "33", "REFUSE": "31"}


# --------------------------------------------------------------------------
# Tools: everything the agent is allowed to do
# --------------------------------------------------------------------------


class Tools:
    def __init__(self, base_url: str, mandate_id: str, auto_confirm: bool, wait_seconds: int):
        self.api = base_url.rstrip("/") + "/api"
        self.mandate_id = mandate_id
        self.auto_confirm = auto_confirm
        self.wait_seconds = wait_seconds
        self.catalog = json.loads(CATALOG_PATH.read_text(encoding="utf-8"))

    def dispatch(self, name: str, args: dict) -> dict:
        handler = getattr(self, name, None)
        if handler is None or name.startswith("_") or name == "dispatch":
            return {"error": f"unknown tool {name}"}
        return handler(**args)

    def search_products(self, query: str) -> dict:
        words = [w for w in query.lower().split() if len(w) > 2]
        hits = [p for p in self.catalog if any(w in (p["name"] + " " + p["category"] + " " + p["tags"]).lower() for w in words)]
        hits = hits or self.catalog
        hits.sort(key=lambda p: -p["rating"])
        say("tool", f"search_products({query!r}) -> {len(hits)} results", "36")
        return {
            "products": [
                {k: p[k] for k in ("id", "name", "merchant", "price_eur", "rating", "delivery_days")} for p in hits
            ]
        }

    def request_purchase(self, product_id: str, reason: str = "") -> dict:
        product = next((p for p in self.catalog if p["id"] == product_id), None)
        if product is None:
            return {"error": f"unknown product {product_id}"}
        body = {
            "mandate_id": self.mandate_id,
            "merchant": product["merchant"],
            "merchant_country": product["merchant_country"],
            "amount_cents": round(product["price_eur"] * 100),
            "currency": "EUR",
            "category": product["category"],
            "delivery_days": product["delivery_days"],
            "description": f"{product['name']}. {reason}".strip(),
            "kind": "purchase",
        }
        say("agent", f"asks AgentShield: {product['name']} at {product['merchant']}, EUR {product['price_eur']:.2f}", "35")
        status, res = http_json("POST", f"{self.api}/actions", body)
        if status >= 400:
            return {"error": res}
        decision = res["decision"]
        failed = [c["label"] for c in res.get("checks", []) if not c.get("passed")]
        say("AgentShield", f"{paint(decision, DECISION_COLOR.get(decision, '0'))} - {res['reasons'][0]}", "1")
        out = {
            "decision": decision,
            "action_id": res["action"]["id"],
            "reasons": res["reasons"],
            "failed_checks": failed,
            "audit_seq": res.get("audit", {}).get("seq"),
        }
        if res.get("approval"):
            out["approval_id"] = res["approval"]["id"]
            out["approval_expires_at"] = res["approval"]["expires_at"]
        return out

    def wait_for_owner_approval(self, approval_id: str) -> dict:
        if self.auto_confirm:
            say("owner", "confirms the purchase from the AgentShield dashboard (demo: --auto-confirm)", "34")
            http_json("POST", f"{self.api}/approvals/{approval_id}/confirm", {"decided_by": "Owner", "note": "Confirmed during demo"})
        else:
            say("agent", f"waiting up to {self.wait_seconds}s for the owner to decide in the dashboard...", "35")
        deadline = time.time() + self.wait_seconds
        while True:
            _, res = http_json("GET", f"{self.api}/approvals")
            approval = next((a for a in res.get("approvals", []) if a["id"] == approval_id), None)
            status = (approval or {}).get("status", "unknown")
            if status != "pending" or time.time() >= deadline:
                say("AgentShield", f"approval is {status}", "1")
                return {"approval_id": approval_id, "status": status}
            time.sleep(2)

    def create_paypal_checkout(self, action_id: str) -> dict:
        status, res = http_json("POST", f"{self.api}/actions/{action_id}/paypal/order", {})
        if status >= 400:
            err = res.get("error", res)
            say("AgentShield", f"checkout blocked: {err.get('code', status)}", "31")
            return {"blocked": True, "status": status, "error": err}
        order = res["order"]
        say("PayPal", f"{res.get('mode', '')} order {order['paypal_order_id']} {order['status']}", "34")
        return {
            "paypal_order_id": order["paypal_order_id"],
            "status": order["status"],
            "approve_url": order["approve_url"],
            "mode": res.get("mode"),
        }


# --------------------------------------------------------------------------
# Brains
# --------------------------------------------------------------------------


def run_openai(task: str, tools: Tools, model: str, max_steps: int = 12) -> str:
    base = os.environ.get("OPENAI_BASE_URL", "https://api.openai.com/v1").rstrip("/")
    headers = {"Authorization": f"Bearer {os.environ['OPENAI_API_KEY']}"}
    messages = [{"role": "system", "content": SYSTEM_PROMPT}, {"role": "user", "content": task}]
    for _ in range(max_steps):
        status, res = http_json(
            "POST",
            f"{base}/chat/completions",
            {"model": model, "messages": messages, "tools": TOOLS, "temperature": 0.2},
            headers,
            timeout=60,
        )
        if status >= 400:
            raise RuntimeError(f"OpenAI API error {status}: {res}")
        msg = res["choices"][0]["message"]
        messages.append(msg)
        calls = msg.get("tool_calls") or []
        if not calls:
            return msg.get("content") or ""
        if msg.get("content"):
            say("agent", msg["content"], "35")
        for call in calls:
            name = call["function"]["name"]
            args = json.loads(call["function"].get("arguments") or "{}")
            result = tools.dispatch(name, args)
            messages.append({"role": "tool", "tool_call_id": call["id"], "content": json.dumps(result)})
    return "Stopped: too many steps."


def run_offline(task: str, tools: Tools) -> str:
    """Deterministic planner that follows the same policy as the LLM prompt."""
    say("agent", "offline planner (no LLM): best-rated first, adapt after a refusal", "35")
    products = tools.search_products(task)["products"]
    tried: set[str] = set()
    limit: float | None = None
    for product in products:
        if product["id"] in tried or (limit is not None and product["price_eur"] > limit):
            continue
        tried.add(product["id"])
        result = tools.request_purchase(product["id"], "Best-rated option that matches the request.")
        decision = result.get("decision")
        if decision == "REFUSE":
            if any("per-transaction" in c or "budget" in c for c in result["failed_checks"]):
                limit = product["price_eur"] - 0.01
                say("agent", "refused on price: looking for a cheaper option that fits the mandate", "35")
            continue
        if decision == "ESCALATE":
            approval = tools.wait_for_owner_approval(result["approval_id"])
            if approval["status"] != "confirmed":
                return f"The owner did not confirm {product['name']} (status: {approval['status']}). Nothing was paid."
        checkout = tools.create_paypal_checkout(result["action_id"])
        if checkout.get("blocked"):
            continue
        return (
            f"Done. {product['name']} from {product['merchant']} for EUR {product['price_eur']:.2f} "
            f"was approved by AgentShield. PayPal checkout: {checkout['approve_url']}"
        )
    return "I could not find a product that fits the owner's mandate. Nothing was paid."


# --------------------------------------------------------------------------
# Entry point
# --------------------------------------------------------------------------


def find_mandate(api: str, hint: str) -> str:
    _, res = http_json("GET", f"{api}/mandates")
    mandates = res.get("mandates", [])
    for m in mandates:
        if hint.lower() in m.get("intent", "").lower():
            return m["id"]
    if not mandates:
        sys.exit("No mandate found. Start the backend with AGENTSHIELD_SEED=true or create a mandate first.")
    return mandates[0]["id"]


def load_backend_env() -> None:
    """Read OPENAI_* settings from backend/.env so keys live in one place.

    Variables already set in the environment always win.
    """
    env_file = Path(__file__).resolve().parent.parent / "backend" / ".env"
    if not env_file.is_file():
        return
    for line in env_file.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        key, value = key.strip(), value.strip().strip('"').strip("'")
        if key.startswith("OPENAI_") and value and key not in os.environ:
            os.environ[key] = value


def main() -> None:
    load_backend_env()
    parser = argparse.ArgumentParser(description="AI shopping agent that pays only through AgentShield.")
    parser.add_argument("task", nargs="?", default=DEFAULT_TASK)
    parser.add_argument("--base-url", default=os.environ.get("AGENTSHIELD_URL", "http://127.0.0.1:8080"))
    parser.add_argument("--mandate", help="Mandate id (default: the seeded headphones mandate)")
    parser.add_argument("--mandate-hint", default="headphones", help="Text used to pick a mandate by intent")
    parser.add_argument("--offline", action="store_true", help="Use the deterministic planner instead of an LLM")
    parser.add_argument("--auto-confirm", action="store_true", help="Demo only: confirm escalations as the owner")
    parser.add_argument("--wait", type=int, default=120, help="Seconds to wait for an owner decision")
    parser.add_argument("--model", default=os.environ.get("OPENAI_MODEL", "gpt-4o-mini"))
    args = parser.parse_args()

    api = args.base_url.rstrip("/") + "/api"
    status, health = http_json("GET", f"{api}/health")
    if status != 200:
        sys.exit(f"AgentShield is not reachable at {args.base_url}. Start the backend first (cd backend && cargo run).")

    mandate_id = args.mandate or find_mandate(api, args.mandate_hint)
    tools = Tools(args.base_url, mandate_id, args.auto_confirm, args.wait)
    use_llm = not args.offline and bool(os.environ.get("OPENAI_API_KEY"))

    say("user", args.task, "1")
    say("system", f"AgentShield {health.get('version', '')} | PayPal mode: {health['modes']['paypal']} | "
        f"brain: {'OpenAI ' + args.model if use_llm else 'offline planner'}", "90")
    answer = run_openai(args.task, tools, args.model) if use_llm else run_offline(args.task, tools)
    say("agent", answer, "35")


if __name__ == "__main__":
    main()
