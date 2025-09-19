pub mod session;
pub mod task;

// Re-export main session types
pub use session::{SessionManager, SessionManagerConfig, SessionInitOptions, SessionMetadata};

// Re-export main task types
pub use task::{TaskManager, TaskManagerConfig, Task, TaskSpec, TaskStatus, TaskPriority};
