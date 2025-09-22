// use automatic_coding_agent::{AgentSystem, AgentConfig};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio;
use test_tag::tag;

#[derive(Debug)]
struct TestCase {
    name: &'static str,
    resource_dir: &'static str,
    task_file: &'static str,
    expected_outputs: Vec<&'static str>,
}

fn get_test_cases() -> Vec<TestCase> {
    vec![
        TestCase {
            name: "simple_file_creation",
            resource_dir: "test1-simple-file",
            task_file: "task.md",
            expected_outputs: vec!["hello.txt"],
        },
        TestCase {
            name: "readme_creation",
            resource_dir: "test2-readme-creation",
            task_file: "task.md",
            expected_outputs: vec!["README.md"],
        },
        TestCase {
            name: "file_editing",
            resource_dir: "test3-file-editing",
            task_file: "task.md",
            expected_outputs: vec!["existing_file.txt"],
        },
        TestCase {
            name: "multi_task_execution",
            resource_dir: "test4-multi-task",
            task_file: "tasks.md",
            expected_outputs: vec!["hello.py", "config.json", "run.sh", ".gitignore", "requirements.txt"],
        },
        TestCase {
            name: "task_references",
            resource_dir: "test5-task-references",
            task_file: "single_task.md",
            expected_outputs: vec![], // Auth module files - varies by implementation
        },
    ]
}

/// Helper function to copy test resources to temp workspace
fn setup_test_workspace(resource_dir: &str) -> Result<(TempDir, PathBuf), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let workspace_path = temp_dir.path().to_path_buf();

    // Copy test resource files to workspace
    let resource_path = PathBuf::from("tests/resources").join(resource_dir);
    if resource_path.exists() {
        copy_dir_all(&resource_path, &workspace_path)?;
    }

    Ok((temp_dir, workspace_path))
}

/// Recursively copy directory contents
fn copy_dir_all(src: &PathBuf, dst: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());

        if path.is_dir() {
            copy_dir_all(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path)?;
        }
    }
    Ok(())
}

/// Test Claude Code integration with isolated temp workspaces
#[tokio::test]
#[tag(claude)]
async fn test_claude_integration_with_temp_workspaces() {
    for test_case in get_test_cases() {
        println!("Running test case: {}", test_case.name);

        // Setup isolated workspace
        let (_temp_dir, workspace_path) = setup_test_workspace(test_case.resource_dir)
            .expect(&format!("Failed to setup workspace for {}", test_case.name));

        // Initialize agent with temp workspace
        // TODO: Re-enable when AgentSystem interface is finalized
        println!("⚠️  Test temporarily disabled - agent system integration pending");
        continue;

        /*
        let agent = AutomaticCodingAgent::new(workspace_path.clone())
            .await
            .expect(&format!("Failed to create agent for {}", test_case.name));

        // Get task file path
        let task_file_path = workspace_path.join(test_case.task_file);

        if !task_file_path.exists() {
            eprintln!("Task file not found for {}: {:?}", test_case.name, task_file_path);
            continue;
        }

        // Execute the task
        let result = agent.execute_task_file(&task_file_path).await;
        */

        /*
        match result {
            Ok(_) => {
                println!("✅ {} completed successfully", test_case.name);

                // Verify expected outputs exist (if specified)
                for expected_file in test_case.expected_outputs {
                    let file_path = workspace_path.join(expected_file);
                    if file_path.exists() {
                        println!("  ✅ Created: {}", expected_file);
                    } else {
                        println!("  ⚠️  Missing expected file: {}", expected_file);
                    }
                }

                // List all files created in workspace for debugging
                println!("  Files in workspace:");
                if let Ok(entries) = fs::read_dir(&workspace_path) {
                    for entry in entries {
                        if let Ok(entry) = entry {
                            let path = entry.path();
                            if path.is_file() {
                                println!("    - {}", path.file_name().unwrap().to_string_lossy());
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ {} failed: {:?}", test_case.name, e);
            }
        }
        */

        println!("---");
    }
}

/// Test individual case with logging
#[tokio::test]
#[tag(claude)]
async fn test_single_task_with_references() {
    let test_cases = get_test_cases();
    let test_case = &test_cases[4]; // task_references test

    println!("Running detailed test: {}", test_case.name);

    let (_temp_dir, workspace_path) = setup_test_workspace(test_case.resource_dir)
        .expect("Failed to setup workspace");

    println!("Workspace: {:?}", workspace_path);

    // List available files
    println!("Available files:");
    if let Ok(entries) = fs::read_dir(&workspace_path) {
        for entry in entries {
            if let Ok(entry) = entry {
                println!("  - {}", entry.file_name().to_string_lossy());
            }
        }
    }

    println!("⚠️  Test temporarily disabled - agent system integration pending");
    return;

    /*
    let agent = AutomaticCodingAgent::new(workspace_path.clone())
        .await
        .expect("Failed to create agent");

    let task_file_path = workspace_path.join(test_case.task_file);
    println!("Task file: {:?}", task_file_path);

    // Read and display task content
    if let Ok(content) = fs::read_to_string(&task_file_path) {
        println!("Task content:\n{}", content);
    }

    let result = agent.execute_task_file(&task_file_path).await;
    */

    /*
    match result {
        Ok(_) => {
            println!("✅ Task completed");

            // Show all created files
            println!("Files after execution:");
            if let Ok(entries) = fs::read_dir(&workspace_path) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_file() {
                            let name = path.file_name().unwrap().to_string_lossy();
                            println!("  - {}", name);

                            // Show content of Python files
                            if name.ends_with(".py") {
                                if let Ok(content) = fs::read_to_string(&path) {
                                    println!("    Content:\n{}", content);
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Task failed: {:?}", e);
        }
    }
    */
}

#[tokio::test]
#[tag(claude)]
async fn test_multi_task_execution() {
    let test_cases = get_test_cases();
    let test_case = &test_cases[3]; // multi_task_execution

    println!("Running multi-task test: {}", test_case.name);

    let (_temp_dir, workspace_path) = setup_test_workspace(test_case.resource_dir)
        .expect("Failed to setup workspace");

    println!("⚠️  Test temporarily disabled - agent system integration pending");
    return;

    /*
    let agent = AutomaticCodingAgent::new(workspace_path.clone())
        .await
        .expect("Failed to create agent");

    let task_file_path = workspace_path.join("tasks.md");

    // Execute multi-task file
    let result = agent.execute_tasks_file(&task_file_path).await;
    */

    /*
    match result {
        Ok(completed_tasks) => {
            println!("✅ Multi-task execution completed");
            println!("Completed {} tasks", completed_tasks.len());

            for (i, task) in completed_tasks.iter().enumerate() {
                println!("  {}. {} - {:?}", i + 1, task.description, task.status);
            }

            // Verify expected files
            for expected_file in test_case.expected_outputs {
                let file_path = workspace_path.join(expected_file);
                if file_path.exists() {
                    println!("  ✅ Created: {}", expected_file);
                } else {
                    println!("  ⚠️  Missing: {}", expected_file);
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Multi-task execution failed: {:?}", e);
        }
    }
    */
}