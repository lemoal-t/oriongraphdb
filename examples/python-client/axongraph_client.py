"""
AxonGraph Python Client

Simple HTTP client for calling the AxonGraph context compilation service.
"""

import httpx
from typing import Optional, Dict, Any


class AxonGraphClient:
    """Client for AxonGraph context compilation service."""
    
    def __init__(self, base_url: Optional[str] = None):
        """
        Initialize client.
        
        Args:
            base_url: Base URL of AxonGraph server. If None, loads from config.
        """
        if base_url is None:
            try:
                from orion.config import load_config
                cfg = load_config()
                base_url = cfg.axon_url
            except ImportError:
                # Fallback if orion package not installed
                base_url = "http://localhost:8081"
        
        self.base_url = base_url
        self._client = httpx.Client(timeout=30.0)
    
    def health(self) -> Dict[str, Any]:
        """Check server health."""
        response = self._client.get(f"{self.base_url}/health")
        response.raise_for_status()
        return response.json()
    
    def compile_workingset(
        self,
        intent: str,
        budget_tokens: int = 6000,
        workstream: Optional[str] = None,
        session_id: Optional[str] = None,
        user_id: Optional[str] = None,
        explain: bool = True,
        prefer_stages: Optional[list] = None,
    ) -> Dict[str, Any]:
        """
        Compile an optimal context working set.
        
        Args:
            intent: What context do you need? E.g., "find rollback procedures"
            budget_tokens: Maximum tokens for context (default: 6000)
            workstream: Optional workstream to scope search (e.g., "ws-migration")
            session_id: Optional session ID for session-aware context
            user_id: Optional user ID for memory-aware context
            prefer_stages: Optional list of stage labels to bias selection
                (e.g., ["memory_decisions", "design"])
            explain: Whether to include selection explanations (default: True)
        
        Returns:
            CompileResponse dict with:
                - workingset: { spans: [...], total_tokens: int }
                - stats: { candidates_generated, token_utilization, ... }
                - rationale: [SpanExplanation, ...] (if explain=True)
        
        Raises:
            httpx.HTTPError: If request fails
        """
        payload = {
            "intent": intent,
            "budget_tokens": budget_tokens,
            "workstream": workstream,
            "explain": explain,
        }
        
        # Add optional session/user context
        if session_id:
            payload["session_id"] = session_id
        if user_id:
            payload["user_id"] = user_id
        if prefer_stages:
            # Pass through as a soft preference hint; the server
            # may use this to bias scoring via SoftPreferences.
            payload["prefer_stages"] = prefer_stages
        
        response = self._client.post(
            f"{self.base_url}/compile_workingset",
            json=payload
        )
        response.raise_for_status()
        return response.json()
    
    def close(self):
        """Close the HTTP client."""
        self._client.close()
    
    def __enter__(self):
        return self
    
    def __exit__(self, *args):
        self.close()


# Example usage
if __name__ == "__main__":
    client = AxonGraphClient()
    
    # Check health
    print("Health check:", client.health())
    
    # Compile working set
    result = client.compile_workingset(
        intent="draft a cutover plan with rollback strategy",
        budget_tokens=6000,
        workstream="ws-migration"
    )
    
    print(f"\n✓ Compiled working set:")
    print(f"  Spans: {len(result['workingset']['spans'])}")
    print(f"  Tokens: {result['workingset']['total_tokens']}")
    print(f"  Utilization: {result['stats']['token_utilization'] * 100:.1f}%")
    
    for i, span in enumerate(result['workingset']['spans'], 1):
        print(f"\n  Span {i}: {span['span_ref']['span_id']}")
        print(f"    Source: {span['span_ref']['doc_version_id']}")
        print(f"    Tokens: {span['span_ref']['token_cost']}")
