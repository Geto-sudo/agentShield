"""HTTP client for communicating with the AgentShield backend."""

import json
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.parse import urlencode
from urllib.request import Request, urlopen

API_BASE_URL = "http://127.0.0.1:8080"
REQUEST_TIMEOUT = 5


def _send_request(
    method: str,
    endpoint: str,
    payload: dict[str, Any] | None = None,
) -> Any:
    url = f"{API_BASE_URL}{endpoint}"
    body = None
    headers = {"Accept": "application/json"}

    if payload is not None:
        body = json.dumps(payload).encode("utf-8")
        headers["Content-Type"] = "application/json"

    request = Request(url, data=body, headers=headers, method=method)

    try:
        with urlopen(request, timeout=REQUEST_TIMEOUT) as response:
            response_body = response.read().decode("utf-8")
            return json.loads(response_body)

    except HTTPError as error:
        details = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(
            f"Backend HTTP error: {error.code}. Details: {details}"
        ) from error

    except URLError as error:
        raise RuntimeError(
            "Unable to reach the AgentShield backend. "
            "Make sure the Rust server is running."
        ) from error

    except (json.JSONDecodeError, UnicodeDecodeError) as error:
        raise RuntimeError(
            "The backend returned an invalid JSON response."
        ) from error


def get_json(endpoint: str) -> Any:
    return _send_request("GET", endpoint)


def post_json(endpoint: str, payload: dict[str, Any]) -> Any:
    return _send_request("POST", endpoint, payload)


# AGENTS

def get_agents() -> list[dict[str, Any]]:
    data = get_json("/api/agents")
    return data["agents"]


# ACTIONS

def get_actions() -> list[dict[str, Any]]:
    data = get_json("/api/actions")
    return data["actions"]


def propose_action(payload: dict[str, Any]) -> dict[str, Any]:
    return post_json("/api/actions", payload)


# MANDATES

def get_mandates() -> list[dict[str, Any]]:
    data = get_json("/api/mandates")
    return data["mandates"]


def create_mandate(payload: dict[str, Any]) -> dict[str, Any]:
    data = post_json("/api/mandates", payload)
    return data["mandate"]


# APPROVALS

def get_approvals(status="pending", limit=50):
    query_parameters = urlencode({
        "status": status,
        "limit": limit,
    })
    data = get_json(f"/api/approvals?{query_parameters}")
    return data["approvals"]


def confirm_approval(
    approval_id,
    decided_by="Owner",
    note=None,
):
    payload = {
        "decided_by": decided_by,
        "note": note,
    }
    return post_json(
        f"/api/approvals/{approval_id}/confirm",
        payload,
    )


def deny_approval(
    approval_id,
    decided_by="Owner",
    note=None,
):
    payload = {
        "decided_by": decided_by,
        "note": note,
    }
    return post_json(
        f"/api/approvals/{approval_id}/deny",
        payload,
    )


# WALLET

def get_wallet(agent_id: str) -> dict[str, Any]:
    """Retrieve one agent's wallet, quote and recent ledger entries."""
    data = get_json(f"/api/wallet/{agent_id}")
    return data


def get_reserve() -> dict[str, Any]:
    """Retrieve the shared reserve balance and its recent ledger."""
    return get_json("/api/wallet/reserve")