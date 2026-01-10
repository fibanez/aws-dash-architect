#![warn(clippy::all, rust_2018_idioms)]

//! Task-Agent Integration Tests
//!
//! These tests verify the complete task-agent system end-to-end:
//! - TaskManager agent creation
//! - Worker agent spawning via start-task tool
//! - Agent communication and coordination
//! - UI event integration

use awsdash::app::agent_framework::{
    clear_current_agent_id, get_agent_creation_receiver, get_current_agent_id,
    get_ui_event_receiver, init_agent_creation_channel, init_ui_event_channel,
    request_agent_creation, set_current_agent_id, take_response_channel, AgentCreationRequest,
    AgentCreationResponse, AgentId, AgentInstance, AgentMetadata, AgentModel, AgentType,
    AgentUIEvent,
};
use chrono::Utc;
use std::time::Duration;

/// Helper to create test metadata
fn create_test_metadata(name: &str) -> AgentMetadata {
    AgentMetadata {
        name: name.to_string(),
        description: "Test agent".to_string(),
        model: AgentModel::default(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[test]
fn test_agent_creation_channel_initialization() {
    // Initialize channels
    init_agent_creation_channel();
    init_ui_event_channel();

    // Verify we can get sender/receiver
    let sender = awsdash::app::agent_framework::get_agent_creation_sender();
    let receiver = get_agent_creation_receiver();

    // Send a test request
    let parent_id = AgentId::new();
    let (request, response_receiver) =
        AgentCreationRequest::new("Test".to_string(), "Test task".to_string(), None, parent_id);

    sender.send(request.clone()).unwrap();

    // Verify we can receive it
    let received = receiver.lock().unwrap().try_recv().unwrap();
    assert_eq!(received.request_id, request.request_id);
    assert_eq!(received.task_description, "Test task");
}

#[test]
fn test_request_response_matching() {
    init_agent_creation_channel();

    let parent_id = AgentId::new();
    let (request, response_receiver) =
        AgentCreationRequest::new("Test".to_string(), "Test task".to_string(), None, parent_id);

    let request_id = request.request_id;

    // Get the response channel
    let response_sender = take_response_channel(request_id).unwrap();

    // Send a response
    let agent_id = AgentId::new();
    let response = AgentCreationResponse::success(agent_id);
    response_sender.send(response.clone()).unwrap();

    // Verify the requester receives it
    let received = response_receiver
        .recv_timeout(Duration::from_millis(100))
        .unwrap();
    assert_eq!(received.agent_id, agent_id);
    assert!(received.success);
}

#[test]
fn test_thread_local_agent_context() {
    // Initially no agent context
    assert!(get_current_agent_id().is_none());

    // Set agent context
    let agent_id = AgentId::new();
    set_current_agent_id(agent_id);

    // Verify it's set
    assert_eq!(get_current_agent_id(), Some(agent_id));

    // Clear it
    clear_current_agent_id();
    assert!(get_current_agent_id().is_none());
}

#[test]
fn test_ui_event_communication() {
    init_ui_event_channel();

    let agent_id = AgentId::new();

    // Send UI event
    awsdash::app::agent_framework::send_ui_event(AgentUIEvent::SwitchToAgent(agent_id)).unwrap();

    // Receive it
    let receiver = get_ui_event_receiver();
    let event = receiver.lock().unwrap().try_recv().unwrap();

    match event {
        AgentUIEvent::SwitchToAgent(id) => assert_eq!(id, agent_id),
        _ => panic!("Expected SwitchToAgent event"),
    }
}

#[test]
fn test_agent_creation_with_parent_tracking() {
    // Create parent TaskManager agent
    let parent_metadata = create_test_metadata("Parent TaskManager");
    let parent_agent = AgentInstance::new(parent_metadata, AgentType::TaskManager);
    let parent_id = parent_agent.id();

    // Create child TaskWorker agent
    let child_metadata = create_test_metadata("Child TaskWorker");
    let child_agent = AgentInstance::new(
        child_metadata,
        AgentType::TaskWorker {
            parent_id: parent_id,
        },
    );
    let child_id = child_agent.id();

    // Verify parent-child relationship
    assert_ne!(parent_id, child_id);
    match child_agent.agent_type() {
        AgentType::TaskWorker { parent_id: pid } => {
            assert_eq!(pid, parent_id);
        }
        _ => panic!("Expected TaskWorker agent type"),
    }
}

/// Integration test simulating the complete flow:
/// 1. Create TaskManager agent
/// 2. Set it as current context
/// 3. Request agent creation (simulating start-task tool)
/// 4. Process request (simulating AgentManagerWindow)
/// 5. Verify worker created with correct parent_id
#[test]
fn test_complete_agent_spawning_flow() {
    init_agent_creation_channel();
    init_ui_event_channel();

    // Create parent TaskManager agent
    let parent_metadata = create_test_metadata("Integration Test Manager");
    let parent_agent = AgentInstance::new(parent_metadata, AgentType::TaskManager);
    let parent_id = parent_agent.id();

    // Set parent as current context (simulating tool execution)
    set_current_agent_id(parent_id);

    // Request agent creation in background thread
    let parent_id_clone = parent_id;
    let spawner_thread = std::thread::spawn(move || {
        // This simulates the start-task tool calling request_agent_creation
        request_agent_creation(
            "List EC2".to_string(),
            "List all EC2 instances".to_string(),
            Some("JSON array".to_string()),
            parent_id_clone,
        )
    });

    // Process request (simulating AgentManagerWindow)
    std::thread::sleep(Duration::from_millis(50)); // Let request arrive

    let receiver = get_agent_creation_receiver();
    let request = receiver.lock().unwrap().try_recv().unwrap();

    // Verify request
    assert_eq!(request.task_description, "List all EC2 instances");
    assert_eq!(
        request.expected_output_format,
        Some("JSON array".to_string())
    );
    assert_eq!(request.parent_id, parent_id);

    // Create worker agent
    let worker_metadata = create_test_metadata("Worker 1");
    let worker_agent = AgentInstance::new(
        worker_metadata,
        AgentType::TaskWorker {
            parent_id: request.parent_id,
        },
    );
    let worker_id = worker_agent.id();

    // Send response
    let response_sender = take_response_channel(request.request_id).unwrap();
    let response = AgentCreationResponse::success(worker_id);
    response_sender.send(response).unwrap();

    // Verify spawner received the worker ID
    let result = spawner_thread.join().unwrap();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), worker_id);

    // Cleanup
    clear_current_agent_id();
}

/// Integration test simulating multiple concurrent worker spawns
#[test]
fn test_multiple_concurrent_worker_spawns() {
    init_agent_creation_channel();
    init_ui_event_channel();

    // Create parent
    let parent_metadata = create_test_metadata("Multi-Task Manager");
    let parent_agent = AgentInstance::new(parent_metadata, AgentType::TaskManager);
    let parent_id = parent_agent.id();

    set_current_agent_id(parent_id);

    // Spawn 3 workers concurrently
    let tasks = vec![
        "List EC2 instances",
        "List S3 buckets",
        "List RDS databases",
    ];

    let mut spawn_threads = vec![];
    for task in tasks.iter() {
        let task_str = task.to_string();
        let pid = parent_id;
        let handle = std::thread::spawn(move || {
            request_agent_creation("Task".to_string(), task_str, None, pid)
        });
        spawn_threads.push(handle);
    }

    // Process all requests
    std::thread::sleep(Duration::from_millis(100)); // Let all requests arrive

    let receiver = get_agent_creation_receiver();
    let mut worker_ids = vec![];

    for i in 0..3 {
        let request = receiver.lock().unwrap().try_recv().unwrap();

        // Create worker
        let worker_metadata = create_test_metadata(&format!("Worker {}", i + 1));
        let worker_agent = AgentInstance::new(
            worker_metadata,
            AgentType::TaskWorker {
                parent_id: request.parent_id,
            },
        );
        let worker_id = worker_agent.id();
        worker_ids.push(worker_id);

        // Send response
        let response_sender = take_response_channel(request.request_id).unwrap();
        let response = AgentCreationResponse::success(worker_id);
        response_sender.send(response).unwrap();
    }

    // Verify all spawns succeeded
    for handle in spawn_threads {
        let result = handle.join().unwrap();
        assert!(result.is_ok());
        assert!(worker_ids.contains(&result.unwrap()));
    }

    clear_current_agent_id();
}

/// Integration test for error handling: parent not found
#[test]
fn test_agent_creation_parent_not_found_error() {
    init_agent_creation_channel();

    let non_existent_parent = AgentId::new();

    // Request creation with non-existent parent
    let spawner_thread = std::thread::spawn(move || {
        request_agent_creation(
            "Test".to_string(),
            "Test task".to_string(),
            None,
            non_existent_parent,
        )
    });

    std::thread::sleep(Duration::from_millis(50));

    let receiver = get_agent_creation_receiver();
    let request = receiver.lock().unwrap().try_recv().unwrap();

    // Simulate AgentManagerWindow rejecting because parent doesn't exist
    let response_sender = take_response_channel(request.request_id).unwrap();
    let response = AgentCreationResponse::error(
        AgentId::new(),
        format!("Parent agent {} not found", request.parent_id),
    );
    response_sender.send(response).unwrap();

    // Verify spawner received error
    let result = spawner_thread.join().unwrap();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

/// Integration test for timeout when no one processes the request
#[test]
fn test_agent_creation_timeout() {
    init_agent_creation_channel();

    let parent_id = AgentId::new();
    set_current_agent_id(parent_id);

    // Request creation but don't process it (no AgentManagerWindow)
    let result =
        request_agent_creation("Test".to_string(), "Test task".to_string(), None, parent_id);

    // Should timeout after 5 seconds
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Failed to receive"));

    clear_current_agent_id();
}

/// Integration test verifying UI events are sent after agent creation
#[test]
fn test_ui_event_after_agent_creation() {
    init_agent_creation_channel();
    init_ui_event_channel();

    let parent_id = AgentId::new();
    set_current_agent_id(parent_id);

    // Spawn agent creation request
    let spawner_thread = std::thread::spawn(move || {
        let result =
            request_agent_creation("Test".to_string(), "Test task".to_string(), None, parent_id);

        // After successful creation, send UI event
        if let Ok(agent_id) = result {
            awsdash::app::agent_framework::send_ui_event(AgentUIEvent::SwitchToAgent(agent_id))
                .ok();
        }

        result
    });

    std::thread::sleep(Duration::from_millis(50));

    // Process request
    let receiver = get_agent_creation_receiver();
    let request = receiver.lock().unwrap().try_recv().unwrap();

    let worker_id = AgentId::new();
    let response_sender = take_response_channel(request.request_id).unwrap();
    response_sender
        .send(AgentCreationResponse::success(worker_id))
        .unwrap();

    // Wait for spawner to complete
    let result = spawner_thread.join().unwrap();
    assert!(result.is_ok());

    // Verify UI event was sent
    let ui_receiver = get_ui_event_receiver();
    let event = ui_receiver.lock().unwrap().try_recv().unwrap();

    match event {
        AgentUIEvent::SwitchToAgent(id) => assert_eq!(id, worker_id),
        _ => panic!("Expected SwitchToAgent event"),
    }

    clear_current_agent_id();
}
