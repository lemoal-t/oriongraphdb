#!/usr/bin/env python3
"""
OrionGraphDB Python Client Example

This example shows how to use the OrionGraphDB HTTP API to compile
optimal context for AI agents.
"""

from oriongraph_client import OrionGraphClient


def main():
    # Initialize client
    client = OrionGraphClient("http://localhost:8081")
    
    # Check server health
    print("🔍 Checking OrionGraphDB server...")
    health = client.health()
    print(f"✓ Server is {health.get('status', 'unknown')}\n")
    
    # Example 1: Simple context compilation
    print("📚 Example 1: Compile context for a coding task")
    result = client.compile_workingset(
        intent="Find error handling patterns in the codebase",
        budget_tokens=4000,
    )
    
    print(f"  Compiled {len(result['workingset']['spans'])} spans")
    print(f"  Total tokens: {result['workingset']['total_tokens']}")
    print(f"  Utilization: {result['stats']['token_utilization'] * 100:.1f}%\n")
    
    # Show first span
    if result['workingset']['spans']:
        span = result['workingset']['spans'][0]
        print(f"  First span: {span['span_ref']['span_id']}")
        print(f"  Source: {span['span_ref']['doc_version_id']}")
        print(f"  Preview: {span['text'][:100]}...\n")
    
    # Example 2: Workstream-scoped search
    print("📂 Example 2: Search within a specific workstream")
    result = client.compile_workingset(
        intent="Find database migration rollback procedures",
        budget_tokens=6000,
        workstream="ws-migration",
    )
    
    print(f"  Found {len(result['workingset']['spans'])} relevant spans")
    print(f"  Token budget: {result['workingset']['total_tokens']}/6000\n")
    
    # Example 3: With explanations
    print("💡 Example 3: Get explanations for selections")
    result = client.compile_workingset(
        intent="Authentication and authorization patterns",
        budget_tokens=3000,
        explain=True,
    )
    
    print(f"  Selected {len(result['workingset']['spans'])} spans with rationale:")
    if 'rationale' in result:
        for i, explanation in enumerate(result['rationale'][:3], 1):
            print(f"  {i}. {explanation}")
    
    print("\n✓ All examples completed!")


if __name__ == "__main__":
    main()

