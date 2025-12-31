#!/usr/bin/env python3
"""
AWS Dash Agent Evaluations CLI Tool

Run AWS Bedrock AgentCore evaluations on AWS Dash agent telemetry data.
Supports interactive selection of evaluators and verbose output.

Usage:
    ./evaluate_agent.py                              # Auto-discover latest awsdash agent
    ./evaluate_agent.py --agent-id awsdash-worker-xxx  # Specify agent ID
    ./evaluate_agent.py --list-agents                # List available awsdash agents
    ./evaluate_agent.py --all                        # Run all evaluators
    ./evaluate_agent.py --evaluators Helpfulness Correctness  # Specific evaluators
    ./evaluate_agent.py --list-sessions              # List available sessions
    ./evaluate_agent.py --session-id <id>            # Evaluate specific session

Requirements:
    - boto3
    - AWS credentials configured
    - Agent telemetry data in CloudWatch (enable "Agent Logging" in AWS Dash)
"""

import argparse
import boto3
import json
import sys
import time
from datetime import datetime, timedelta
from typing import Optional

# Available built-in evaluators with descriptions
BUILTIN_EVALUATORS = {
    "Builtin.Helpfulness": {
        "name": "Helpfulness",
        "description": "Evaluates if the response is helpful and addresses user needs",
        "default": True,
    },
    "Builtin.Correctness": {
        "name": "Correctness",
        "description": "Evaluates if the response is factually correct",
        "default": True,
    },
    "Builtin.Harmfulness": {
        "name": "Harmfulness",
        "description": "Evaluates if the response contains harmful content",
        "default": True,
    },
    "Builtin.Faithfulness": {
        "name": "Faithfulness",
        "description": "Evaluates if the response is faithful to tool outputs",
        "default": False,
    },
    "Builtin.Conciseness": {
        "name": "Conciseness",
        "description": "Evaluates if the response is appropriately concise",
        "default": False,
    },
}


class Colors:
    """ANSI color codes for terminal output"""
    HEADER = '\033[95m'
    BLUE = '\033[94m'
    CYAN = '\033[96m'
    GREEN = '\033[92m'
    YELLOW = '\033[93m'
    RED = '\033[91m'
    BOLD = '\033[1m'
    DIM = '\033[2m'
    RESET = '\033[0m'


def print_header(text: str):
    """Print a formatted header"""
    width = 70
    print(f"\n{Colors.CYAN}{'=' * width}{Colors.RESET}")
    print(f"{Colors.BOLD}{Colors.CYAN}  {text}{Colors.RESET}")
    print(f"{Colors.CYAN}{'=' * width}{Colors.RESET}")


def print_subheader(text: str):
    """Print a formatted subheader"""
    print(f"\n{Colors.BLUE}{'─' * 70}{Colors.RESET}")
    print(f"{Colors.BOLD}  {text}{Colors.RESET}")
    print(f"{Colors.BLUE}{'─' * 70}{Colors.RESET}")


def print_success(text: str):
    """Print success message"""
    print(f"{Colors.GREEN}✓ {text}{Colors.RESET}")


def print_error(text: str):
    """Print error message"""
    print(f"{Colors.RED}✗ {text}{Colors.RESET}")


def print_warning(text: str):
    """Print warning message"""
    print(f"{Colors.YELLOW}⚠ {text}{Colors.RESET}")


def discover_awsdash_agents(logs_client, region: str = 'us-east-1') -> list[dict]:
    """
    Discover available AWS Dash agent log groups.

    Returns list of dicts with agent_id, agent_type, creation_time.
    Sorted by creation time (newest first).
    """
    agents = []
    try:
        paginator = logs_client.get_paginator('describe_log_groups')
        for page in paginator.paginate(logGroupNamePrefix='/aws/bedrock-agentcore/runtimes/awsdash-'):
            for lg in page.get('logGroups', []):
                name = lg['logGroupName']
                # Extract agent ID from log group name
                agent_id = name.replace('/aws/bedrock-agentcore/runtimes/', '')

                # Determine agent type
                if 'manager' in agent_id:
                    agent_type = 'manager'
                elif 'worker' in agent_id:
                    agent_type = 'worker'
                else:
                    agent_type = 'unknown'

                agents.append({
                    'agent_id': agent_id,
                    'agent_type': agent_type,
                    'creation_time': lg.get('creationTime', 0),
                    'log_group': name,
                })
    except Exception as e:
        print_error(f"Error discovering agents: {e}")

    # Sort by creation time (newest first)
    agents.sort(key=lambda x: x['creation_time'], reverse=True)
    return agents


def list_awsdash_agents(logs_client, region: str = 'us-east-1'):
    """List all available AWS Dash agents"""
    print_header("Available AWS Dash Agents")

    agents = discover_awsdash_agents(logs_client, region)

    if not agents:
        print("\n  No AWS Dash agents found.")
        print("  Make sure 'Agent Logging' is enabled in AWS Dash settings.")
        return

    print(f"\n  Found {len(agents)} agent(s):\n")
    print(f"  {'Agent ID':<60} {'Type':<10} {'Created':<20}")
    print(f"  {'-' * 60} {'-' * 10} {'-' * 20}")

    for agent in agents[:20]:  # Show last 20
        created = datetime.fromtimestamp(agent['creation_time'] / 1000).strftime('%Y-%m-%d %H:%M:%S')
        print(f"  {agent['agent_id']:<60} {agent['agent_type']:<10} {created}")


def interactive_select_evaluators() -> list[str]:
    """
    Interactive TUI for selecting evaluators.
    Returns list of selected evaluator IDs.
    """
    print_header("Select Evaluators")
    print(f"\n  Use numbers to toggle selection, 'a' for all, 'n' for none, Enter to confirm\n")

    evaluator_ids = list(BUILTIN_EVALUATORS.keys())
    selected = {eid: info["default"] for eid, info in BUILTIN_EVALUATORS.items()}

    while True:
        # Display current selection
        for i, eid in enumerate(evaluator_ids, 1):
            info = BUILTIN_EVALUATORS[eid]
            checkbox = f"{Colors.GREEN}[X]{Colors.RESET}" if selected[eid] else "[ ]"
            default_marker = f"{Colors.DIM}(default){Colors.RESET}" if info["default"] else ""
            print(f"  {i}. {checkbox} {info['name']:<15} - {info['description']} {default_marker}")

        print(f"\n  {Colors.DIM}Commands: 1-{len(evaluator_ids)}=toggle, a=all, n=none, d=defaults, Enter=confirm{Colors.RESET}")

        try:
            choice = input(f"\n  {Colors.BOLD}Selection:{Colors.RESET} ").strip().lower()
        except (EOFError, KeyboardInterrupt):
            print("\n")
            sys.exit(0)

        if choice == "":
            # Confirm selection
            break
        elif choice == "a":
            # Select all
            selected = {eid: True for eid in evaluator_ids}
        elif choice == "n":
            # Select none
            selected = {eid: False for eid in evaluator_ids}
        elif choice == "d":
            # Reset to defaults
            selected = {eid: info["default"] for eid, info in BUILTIN_EVALUATORS.items()}
        elif choice.isdigit():
            idx = int(choice) - 1
            if 0 <= idx < len(evaluator_ids):
                eid = evaluator_ids[idx]
                selected[eid] = not selected[eid]

        # Clear previous output (move cursor up)
        print(f"\033[{len(evaluator_ids) + 4}A\033[J", end="")

    return [eid for eid, is_selected in selected.items() if is_selected]


def get_cloudwatch_data(
    logs_client,
    agent_id: str,
    time_range_minutes: int = 60,
    session_id: Optional[str] = None
) -> tuple[list, list]:
    """
    Retrieve spans and events from CloudWatch for the specified agent.

    Returns:
        Tuple of (spans, events)
    """
    log_group = f"/aws/bedrock-agentcore/runtimes/{agent_id}"

    end_time = int(time.time() * 1000)
    start_time = end_time - (time_range_minutes * 60 * 1000)

    # Get events from runtime log group
    try:
        response = logs_client.filter_log_events(
            logGroupName=log_group,
            startTime=start_time,
            endTime=end_time,
            limit=500
        )
        all_events = [json.loads(e['message']) for e in response.get('events', [])]
    except logs_client.exceptions.ResourceNotFoundException:
        print_error(f"Log group not found: {log_group}")
        print(f"  Make sure the agent has been run with CloudWatch telemetry enabled.")
        return [], []
    except Exception as e:
        print_error(f"Error querying log group: {e}")
        return [], []

    # Get spans from aws/spans
    try:
        response = logs_client.filter_log_events(
            logGroupName='aws/spans',
            startTime=start_time,
            endTime=end_time,
            limit=500
        )
        all_spans = [json.loads(s['message']) for s in response.get('events', [])]
    except Exception as e:
        print_warning(f"Error querying aws/spans: {e}")
        all_spans = []

    # Filter by session_id if specified
    if session_id:
        all_events = [e for e in all_events if e.get('attributes', {}).get('session.id') == session_id]
        all_spans = [s for s in all_spans if s.get('attributes', {}).get('session.id') == session_id]

    return all_spans, all_events


def list_sessions(logs_client, agent_id: str, time_range_minutes: int = 60):
    """List available sessions for the agent"""
    print_header(f"Available Sessions for {agent_id}")

    spans, events = get_cloudwatch_data(logs_client, agent_id, time_range_minutes)

    if not spans and not events:
        print("\n  No data found in the specified time range.")
        return

    # Extract unique sessions
    sessions = {}
    for span in spans:
        session_id = span.get('attributes', {}).get('session.id')
        if session_id and session_id not in sessions:
            sessions[session_id] = {
                'agent_name': span.get('attributes', {}).get('gen_ai.agent.name', 'Unknown'),
                'timestamp': span.get('startTimeUnixNano', 0),
                'trace_id': span.get('traceId', 'Unknown'),
            }

    if not sessions:
        print("\n  No sessions found.")
        return

    # Sort by timestamp (newest first)
    sorted_sessions = sorted(sessions.items(), key=lambda x: x[1]['timestamp'], reverse=True)

    print(f"\n  Found {len(sorted_sessions)} session(s):\n")
    print(f"  {'Session ID':<40} {'Agent Name':<30} {'Trace ID':<20}")
    print(f"  {'-' * 40} {'-' * 30} {'-' * 20}")

    for session_id, info in sorted_sessions[:20]:  # Show last 20
        ts = datetime.fromtimestamp(info['timestamp'] / 1e9).strftime('%H:%M:%S') if info['timestamp'] else 'N/A'
        print(f"  {session_id:<40} {info['agent_name']:<30} {info['trace_id'][:16]}...")


def find_latest_invoke_agent(spans: list, events: list) -> tuple[Optional[dict], Optional[dict]]:
    """
    Find the latest invoke_agent span and its corresponding log event.

    Returns:
        Tuple of (span, event) or (None, None) if not found
    """
    # Find invoke_agent spans (excluding evaluation spans)
    invoke_spans = [
        s for s in spans
        if s.get('name', '').startswith('invoke_agent ')
        and 'evaluation' not in s.get('name', '').lower()
    ]

    if not invoke_spans:
        return None, None

    # Sort by timestamp (newest first)
    invoke_spans.sort(key=lambda x: x.get('startTimeUnixNano', 0), reverse=True)
    latest_span = invoke_spans[0]

    # Find matching log event
    span_id = latest_span['spanId']
    matching_events = [e for e in events if e.get('spanId') == span_id]

    if not matching_events:
        return latest_span, None

    return latest_span, matching_events[0]


def build_session_spans(spans: list, events: list, trace_id: str) -> list:
    """
    Build a complete sessionSpans array for evaluation.

    Includes all spans and events from the trace (except evaluation spans).
    This ensures evaluators like Faithfulness can see tool calls and outputs.

    Args:
        spans: All spans from CloudWatch
        events: All events from CloudWatch
        trace_id: The trace ID to filter by

    Returns:
        List of spans and events for the sessionSpans parameter
    """
    session_spans = []

    # Get all spans and events for this trace
    trace_spans = [s for s in spans if s.get('traceId') == trace_id]
    trace_events = [e for e in events if e.get('traceId') == trace_id]

    # Sort spans by start time
    trace_spans.sort(key=lambda x: x.get('startTimeUnixNano', 0))

    for span in trace_spans:
        span_name = span.get('name', '')

        # Skip evaluation spans and internal operations only
        if 'evaluation' in span_name.lower():
            continue
        if span_name == 'InternalOperation':
            continue
        # Skip parallel_group wrapper spans (no useful info)
        if 'parallel_group' in span_name:
            continue

        # Include the span
        session_spans.append(span)

        # Add corresponding event if exists
        span_id = span.get('spanId')
        for event in trace_events:
            if event.get('spanId') == span_id:
                session_spans.append(event)
                break

    return session_spans


def run_evaluation(
    agentcore_client,
    evaluator_id: str,
    session_spans: list,
    verbose: bool = False
) -> dict:
    """
    Run a single evaluation and return results.

    Returns:
        Dict with status, score, label, explanation, and token usage
    """
    try:
        response = agentcore_client.evaluate(
            evaluatorId=evaluator_id,
            evaluationInput={
                'sessionSpans': session_spans
            }
        )

        results = response.get('evaluationResults', [])
        if results:
            result = results[0]
            token_usage = result.get('tokenUsage', {})
            return {
                'status': 'SUCCESS',
                'score': result.get('value', 'N/A'),
                'label': result.get('label', 'N/A'),
                'explanation': result.get('explanation', 'N/A'),
                'token_usage': {
                    'input': token_usage.get('inputTokens', 0),
                    'output': token_usage.get('outputTokens', 0),
                    'total': token_usage.get('totalTokens', 0),
                },
                'evaluator_arn': result.get('evaluatorArn', 'N/A'),
            }
        else:
            return {
                'status': 'SUCCESS',
                'score': 'N/A',
                'label': 'No results',
                'explanation': json.dumps(response, indent=2, default=str),
                'token_usage': {'input': 0, 'output': 0, 'total': 0},
                'evaluator_arn': 'N/A',
            }

    except Exception as e:
        error_msg = str(e)
        if hasattr(e, 'response'):
            error_msg = e.response.get('Error', {}).get('Message', str(e))
        return {
            'status': 'FAILED',
            'score': 'N/A',
            'label': 'Error',
            'explanation': error_msg,
            'token_usage': {'input': 0, 'output': 0, 'total': 0},
            'evaluator_arn': 'N/A',
        }


def run_evaluations(
    agent_id: str,
    evaluator_ids: list[str],
    region: str = 'us-east-1',
    time_range_minutes: int = 60,
    session_id: Optional[str] = None,
    verbose: bool = False
):
    """Main function to run evaluations"""

    # Initialize clients
    logs_client = boto3.client('logs', region_name=region)
    agentcore_client = boto3.client('bedrock-agentcore', region_name=region)

    print_header(f"AgentCore Evaluations")
    print(f"\n  Agent ID: {agent_id}")
    print(f"  Region: {region}")
    print(f"  Time Range: Last {time_range_minutes} minutes")
    if session_id:
        print(f"  Session ID: {session_id}")
    print(f"  Evaluators: {len(evaluator_ids)}")

    # Get CloudWatch data
    print_subheader("Retrieving Telemetry Data")
    spans, events = get_cloudwatch_data(logs_client, agent_id, time_range_minutes, session_id)

    print(f"\n  Retrieved {len(spans)} spans and {len(events)} events from CloudWatch")

    if not spans or not events:
        print_error("No telemetry data found. Make sure:")
        print("  1. The agent has been run with CloudWatch telemetry enabled")
        print("  2. The agent ID matches the configured agent")
        print("  3. The time range includes recent agent runs")
        return

    # Find latest invoke_agent span and event
    span, event = find_latest_invoke_agent(spans, events)

    if not span:
        print_error("No invoke_agent spans found in the data")
        return

    if not event:
        print_error(f"No matching log event found for span {span['spanId']}")
        return

    # Display session info
    print_subheader("Session Data")
    print(f"\n  Span Name: {span.get('name', 'N/A')}")
    print(f"  Trace ID: {span.get('traceId', 'N/A')}")
    print(f"  Span ID: {span.get('spanId', 'N/A')}")
    print(f"  Session ID: {span.get('attributes', {}).get('session.id', 'N/A')}")
    print(f"  Scope: {span.get('scope', {}).get('name', 'N/A')}")

    # Show key attributes
    attrs = span.get('attributes', {})
    key_attrs = ['traceloop.span.kind', 'gen_ai.agent.name', 'gen_ai.operation.name']
    shown_attrs = [(k, attrs[k]) for k in key_attrs if k in attrs]
    if shown_attrs:
        print(f"\n  Key Attributes:")
        for key, value in shown_attrs:
            print(f"    {key}: {value}")

    if verbose:
        # Show body preview
        body = event.get('body', {})
        if body.get('input', {}).get('messages'):
            input_msg = body['input']['messages'][0]
            content = input_msg.get('content', '')[:300]
            print(f"\n  Input Preview:")
            print(f"    Role: {input_msg.get('role')}")
            print(f"    Content: {content}...")

        if body.get('output', {}).get('messages'):
            output_msg = body['output']['messages'][0]
            content = output_msg.get('content', '')[:300]
            print(f"\n  Output Preview:")
            print(f"    Role: {output_msg.get('role')}")
            print(f"    Content: {content}...")

    # Build sessionSpans for evaluation (all spans and events from trace)
    trace_id = span.get('traceId')
    session_spans = build_session_spans(spans, events, trace_id)

    # Count what we're sending
    span_count = sum(1 for s in session_spans if 'startTimeUnixNano' in s)
    event_count = len(session_spans) - span_count
    print(f"\n  Session Spans: {span_count} spans + {event_count} events")

    # Run evaluations
    print_subheader("Running Evaluations")

    results = []
    for evaluator_id in evaluator_ids:
        name = BUILTIN_EVALUATORS.get(evaluator_id, {}).get('name', evaluator_id)
        print(f"\n  Evaluating: {name}...", end=" ", flush=True)

        result = run_evaluation(agentcore_client, evaluator_id, session_spans, verbose)
        result['evaluator_id'] = evaluator_id
        result['name'] = name
        results.append(result)

        if result['status'] == 'SUCCESS':
            print_success(f"Score: {result['score']} ({result['label']})")
        else:
            print_error("Failed")

        if verbose:
            # Show token usage
            tokens = result.get('token_usage', {})
            if tokens.get('total', 0) > 0:
                print(f"    Tokens: {tokens['input']} in / {tokens['output']} out ({tokens['total']} total)")

            # Word wrap explanation
            if result['explanation']:
                print(f"\n    Explanation:")
                words = result['explanation'].split()
                line = "      "
                for word in words:
                    if len(line) + len(word) > 75:
                        print(line)
                        line = "      " + word + " "
                    else:
                        line += word + " "
                if line.strip():
                    print(line)

    # Summary
    print_subheader("Results Summary")

    print(f"\n  {'Evaluator':<25} {'Score':<10} {'Label':<25}")
    print(f"  {'-' * 25} {'-' * 10} {'-' * 25}")

    for r in results:
        if r['status'] == 'SUCCESS':
            score_str = str(r['score']) if r['score'] != 'N/A' else '--'
            print(f"  {r['name']:<25} {score_str:<10} {r['label']:<25}")
        else:
            print(f"  {r['name']:<25} {'--':<10} {Colors.RED}(failed){Colors.RESET}")

    success_count = sum(1 for r in results if r['status'] == 'SUCCESS')
    print(f"\n  Total: {success_count}/{len(results)} evaluations successful")
    print(f"  Format: {span.get('scope', {}).get('name', 'unknown')}")

    # Show total token usage if verbose
    if verbose:
        total_tokens = sum(r.get('token_usage', {}).get('total', 0) for r in results)
        if total_tokens > 0:
            total_input = sum(r.get('token_usage', {}).get('input', 0) for r in results)
            total_output = sum(r.get('token_usage', {}).get('output', 0) for r in results)
            print(f"  Total Tokens: {total_input} in / {total_output} out ({total_tokens} total)")

    return results


def main():
    parser = argparse.ArgumentParser(
        description='Run AgentCore evaluations on AWS Dash agent telemetry',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s                                    # Auto-discover latest awsdash agent
  %(prog)s --list-agents                      # List all awsdash agents
  %(prog)s --agent-id awsdash-worker-xxx      # Specify agent
  %(prog)s --all --verbose                    # All evaluators with details
  %(prog)s -e Helpfulness Correctness         # Specific evaluators
  %(prog)s --list-sessions                    # List available sessions
  %(prog)s --session-id abc-123               # Evaluate specific session
        """
    )

    parser.add_argument(
        '--agent-id', '-a',
        default=None,
        help='Agent ID (default: auto-discover latest awsdash agent)'
    )

    parser.add_argument(
        '--list-agents',
        action='store_true',
        help='List available AWS Dash agents and exit'
    )

    parser.add_argument(
        '--region', '-r',
        default='us-east-1',
        help='AWS region (default: us-east-1)'
    )

    parser.add_argument(
        '--time-range', '-t',
        type=int,
        default=60,
        help='Time range in minutes to search for data (default: 60)'
    )

    parser.add_argument(
        '--session-id', '-s',
        help='Specific session ID to evaluate'
    )

    parser.add_argument(
        '--evaluators', '-e',
        nargs='+',
        choices=[info['name'] for info in BUILTIN_EVALUATORS.values()],
        help='Evaluators to run (by short name)'
    )

    parser.add_argument(
        '--all',
        action='store_true',
        help='Run all available evaluators'
    )

    parser.add_argument(
        '--defaults',
        action='store_true',
        help='Use default evaluators (non-interactive)'
    )

    parser.add_argument(
        '--verbose', '-v',
        action='store_true',
        help='Show verbose output including explanations'
    )

    parser.add_argument(
        '--list-sessions',
        action='store_true',
        help='List available sessions and exit'
    )

    parser.add_argument(
        '--list-evaluators',
        action='store_true',
        help='List available evaluators and exit'
    )

    args = parser.parse_args()

    # Initialize logs client early for discovery
    logs_client = boto3.client('logs', region_name=args.region)

    # List agents mode
    if args.list_agents:
        list_awsdash_agents(logs_client, args.region)
        return

    # List evaluators mode
    if args.list_evaluators:
        print_header("Available Built-in Evaluators")
        print()
        for eid, info in BUILTIN_EVALUATORS.items():
            default_marker = "(default)" if info["default"] else ""
            print(f"  {info['name']:<15} - {info['description']} {default_marker}")
        print()
        return

    # Auto-discover agent if not specified
    agent_id = args.agent_id
    if not agent_id:
        print(f"{Colors.CYAN}Auto-discovering AWS Dash agents...{Colors.RESET}")
        agents = discover_awsdash_agents(logs_client, args.region)
        if not agents:
            print_error("No AWS Dash agents found. Make sure 'Agent Logging' is enabled.")
            print("  Use --list-agents to see available agents.")
            return
        # Prefer worker agents (they have the actual task execution)
        worker_agents = [a for a in agents if a['agent_type'] == 'worker']
        if worker_agents:
            agent_id = worker_agents[0]['agent_id']
            print_success(f"Found worker agent: {agent_id}")
        else:
            agent_id = agents[0]['agent_id']
            print_success(f"Found agent: {agent_id}")

    # List sessions mode
    if args.list_sessions:
        list_sessions(logs_client, agent_id, args.time_range)
        return

    # Determine evaluators to run
    if args.all:
        evaluator_ids = list(BUILTIN_EVALUATORS.keys())
    elif args.evaluators:
        # Map short names to full IDs
        name_to_id = {info['name']: eid for eid, info in BUILTIN_EVALUATORS.items()}
        evaluator_ids = [name_to_id[name] for name in args.evaluators]
    elif args.defaults:
        evaluator_ids = [eid for eid, info in BUILTIN_EVALUATORS.items() if info['default']]
    else:
        # Interactive selection
        evaluator_ids = interactive_select_evaluators()

    if not evaluator_ids:
        print_error("No evaluators selected")
        return

    # Run evaluations
    run_evaluations(
        agent_id=agent_id,
        evaluator_ids=evaluator_ids,
        region=args.region,
        time_range_minutes=args.time_range,
        session_id=args.session_id,
        verbose=args.verbose
    )


if __name__ == '__main__':
    main()
