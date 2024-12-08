use actix_web::{web, HttpResponse, Scope};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{Task, TaskConfig, TaskStatus, TaskPriority};
use crate::state::AppState;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/tasks")
            .route("", web::get().to(list_tasks))
            .route("", web::post().to(create_task))
            .route("/{id}/{action}", web::post().to(task_action))
            .route("/clear-completed", web::post().to(clear_completed)),
    );
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub name: String,
    pub task_type: String,
    pub priority: TaskPriority,
    pub description: Option<String>,
    pub config: TaskConfig,
}

async fn list_tasks(state: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let tasks = sqlx::query_as!(
        Task,
        r#"
        SELECT * FROM tasks
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(&state.db)
    .await?;
    
    Ok(HttpResponse::Ok().json(tasks))
}

async fn create_task(
    state: web::Data<AppState>,
    req: web::Json<CreateTaskRequest>,
) -> Result<HttpResponse, AppError> {
    let task = Task {
        id: Uuid::new_v4(),
        name: req.name.clone(),
        task_type: req.task_type.clone(),
        status: TaskStatus::Pending,
        priority: req.priority.clone(),
        description: req.description.clone(),
        progress: 0.0,
        created_at: Utc::now(),
        started_at: None,
        completed_at: None,
        config: req.config.clone(),
    };
    
    sqlx::query!(
        r#"
        INSERT INTO tasks (
            id, name, task_type, status, priority, description,
            progress, created_at, started_at, completed_at, config
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        task.id,
        task.name,
        task.task_type,
        task.status as TaskStatus,
        task.priority as TaskPriority,
        task.description,
        task.progress,
        task.created_at,
        task.started_at,
        task.completed_at,
        serde_json::to_string(&task.config).unwrap()
    )
    .execute(&state.db)
    .await?;
    
    Ok(HttpResponse::Created().json(task))
}

async fn task_action(
    state: web::Data<AppState>,
    path: web::Path<(Uuid, String)>,
) -> Result<HttpResponse, AppError> {
    let (id, action) = path.into_inner();
    
    let mut task = sqlx::query_as!(
        Task,
        r#"SELECT * FROM tasks WHERE id = ?"#,
        id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Task {} not found", id)))?;
    
    match action.as_str() {
        "start" => {
            if task.status != TaskStatus::Pending {
                return Err(AppError::Task("Task is not in pending state".into()));
            }
            task.status = TaskStatus::Running;
            task.started_at = Some(Utc::now());
        }
        "pause" => {
            if task.status != TaskStatus::Running {
                return Err(AppError::Task("Task is not running".into()));
            }
            task.status = TaskStatus::Pending;
        }
        "stop" => {
            if task.status != TaskStatus::Running {
                return Err(AppError::Task("Task is not running".into()));
            }
            task.status = TaskStatus::Failed;
            task.completed_at = Some(Utc::now());
        }
        "retry" => {
            if task.status != TaskStatus::Failed {
                return Err(AppError::Task("Task is not failed".into()));
            }
            task.status = TaskStatus::Pending;
            task.progress = 0.0;
            task.started_at = None;
            task.completed_at = None;
        }
        "delete" => {
            if !matches!(task.status, TaskStatus::Completed | TaskStatus::Failed) {
                return Err(AppError::Task("Can only delete completed or failed tasks".into()));
            }
            sqlx::query!("DELETE FROM tasks WHERE id = ?", id)
                .execute(&state.db)
                .await?;
            return Ok(HttpResponse::NoContent().finish());
        }
        _ => return Err(AppError::BadRequest("Invalid action".into())),
    }
    
    sqlx::query!(
        r#"
        UPDATE tasks
        SET status = ?, progress = ?, started_at = ?, completed_at = ?
        WHERE id = ?
        "#,
        task.status as TaskStatus,
        task.progress,
        task.started_at,
        task.completed_at,
        task.id
    )
    .execute(&state.db)
    .await?;
    
    Ok(HttpResponse::Ok().json(task))
}

async fn clear_completed(state: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    sqlx::query!(
        r#"
        DELETE FROM tasks
        WHERE status IN ('COMPLETED', 'FAILED')
        "#
    )
    .execute(&state.db)
    .await?;
    
    Ok(HttpResponse::NoContent().finish())
} 