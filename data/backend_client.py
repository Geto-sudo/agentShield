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
    """Send an HTTP request and return the decoded JSON response."""

    url = f"{API_BASE_URL}{endpoint}"

    body = None
    headers = {"Accept": "application/json"}

    if payload is not None:
        body = json.dumps(payload).encode("utf-8")
        headers["Content-Type"] = "application/json"

    request = Request(
        url,
        data=body,
        headers=headers,
        method=method,
    )

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
    """Retrieve JSON data from the backend."""

    return _send_request("GET", endpoint)


def post_json(
    endpoint: str,
    payload: dict[str, Any],
) -> Any:
    """Send JSON data to the backend."""

    return _send_request("POST", endpoint, payload)


def get_actions() -> list[dict[str, Any]]:
    """Retrieve all recorded actions."""

    data = get_json("/api/actions")
    return data["actions"]


def get_agents() -> list[dict[str, Any]]:
    """Retrieve all registered agents."""

    data = get_json("/api/agents")
    return data["agents"]


def get_mandates() -> list[dict[str, Any]]:
    """Retrieve all registered spending mandates."""

    data = get_json("/api/mandates")
    return data["mandates"]


def create_mandate(payload: dict[str, Any]) -> dict[str, Any]:
    """Create a spending mandate and return the created mandate."""

    data = post_json("/api/mandates", payload)
    return data["mandate"]


def propose_action(payload: dict[str, Any]) -> dict[str, Any]:
    """Submit a proposed spending action to the backend."""

    return post_json("/api/actions", payload)


def get_approvals(
    status: str = "pending",
    limit: int = 50,
) -> list[dict[str, Any]]:
    """Retrieve approval requests matching the specified filters."""

    query_parameters = urlencode({
        "status": status,
        "limit": limit,
    })

    data = get_json(f"/api/approvals?{query_parameters}")
    return data["approvals"]


def confirm_approval(
    approval_id: str,
    decided_by: str = "Owner",
    note: str | None = None,
) -> dict[str, Any]:
    """Confirm an approval request."""

    payload = {
        "decided_by": decided_by,
        "note": note,
    }

    return post_json(
        f"/api/approvals/{approval_id}/confirm",
        payload,
    )


def deny_approval(
    approval_id: str,
    decided_by: str = "Owner",
    note: str | None = None,
) -> dict[str, Any]:
    """Reject an approval request."""

    payload = {
        "decided_by": decided_by,
        "note": note,
    }

    return post_json(
        f"/api/approvals/{approval_id}/deny",
        payload,
    )