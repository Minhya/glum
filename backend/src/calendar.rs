use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::json;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

const GOOGLE_CALENDAR_ID: &str = "primary";

#[derive(Clone)]
pub struct CalendarClient {
    google_client_id: String,
    google_client_secret: String,
    client: Client,
}

pub struct SyncedCalendarEvent {
    pub calendar_id: String,
    pub google_event_id: String,
    pub google_event_link: Option<String>,
}

pub struct CalendarTodo<'a> {
    pub id: Uuid,
    pub title: &'a str,
    pub completed: bool,
    pub due_date: OffsetDateTime,
    pub due_has_time: bool,
    pub google_event_id: Option<&'a str>,
}

#[derive(Debug)]
pub struct CalendarSyncError {
    message: String,
}

impl CalendarSyncError {
    pub fn user_message(&self) -> &str {
        &self.message
    }
}

impl CalendarClient {
    pub fn new() -> Self {
        Self {
            google_client_id: std::env::var("GOOGLE_CLIENT_ID").unwrap_or_default(),
            google_client_secret: std::env::var("GOOGLE_CLIENT_SECRET").unwrap_or_default(),
            client: Client::new(),
        }
    }

    pub async fn upsert_event(
        &self,
        google_refresh_token: &str,
        todo: CalendarTodo<'_>,
    ) -> Result<SyncedCalendarEvent, CalendarSyncError> {
        let google_token = self.google_access_token(google_refresh_token).await?;
        let event = calendar_event_body(&todo);

        let request = if let Some(event_id) = todo.google_event_id {
            let url = format!(
                "https://www.googleapis.com/calendar/v3/calendars/{}/events/{}",
                GOOGLE_CALENDAR_ID, event_id
            );
            self.client
                .patch(url)
                .bearer_auth(&google_token)
                .json(&event)
        } else {
            let url = format!(
                "https://www.googleapis.com/calendar/v3/calendars/{}/events",
                GOOGLE_CALENDAR_ID
            );
            self.client
                .post(url)
                .bearer_auth(&google_token)
                .json(&event)
        };

        let response = request
            .send()
            .await
            .map_err(|e| sync_error("Google Calendar request failed", e))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(CalendarSyncError {
                message: google_error_message(status, response).await,
            });
        }

        let event = response
            .json::<GoogleEventResponse>()
            .await
            .map_err(|e| sync_error("Google Calendar response was invalid", e))?;

        Ok(SyncedCalendarEvent {
            calendar_id: GOOGLE_CALENDAR_ID.to_string(),
            google_event_id: event.id,
            google_event_link: event.html_link,
        })
    }

    pub async fn delete_event(
        &self,
        google_refresh_token: &str,
        google_event_id: &str,
    ) -> Result<(), CalendarSyncError> {
        let google_token = self.google_access_token(google_refresh_token).await?;
        let url = format!(
            "https://www.googleapis.com/calendar/v3/calendars/{}/events/{}",
            GOOGLE_CALENDAR_ID, google_event_id
        );

        let response = self
            .client
            .delete(url)
            .bearer_auth(google_token)
            .send()
            .await
            .map_err(|e| sync_error("Google Calendar delete failed", e))?;

        if response.status().is_success() || response.status() == StatusCode::NOT_FOUND {
            return Ok(());
        }

        let status = response.status();
        Err(CalendarSyncError {
            message: google_error_message(status, response).await,
        })
    }

    async fn google_access_token(
        &self,
        google_refresh_token: &str,
    ) -> Result<String, CalendarSyncError> {
        if self.google_client_id.is_empty() || self.google_client_secret.is_empty() {
            return Err(CalendarSyncError {
                message: "Google OAuth is not configured.".to_string(),
            });
        }

        let response = self
            .client
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("client_id", self.google_client_id.as_str()),
                ("client_secret", self.google_client_secret.as_str()),
                ("refresh_token", google_refresh_token),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .await
            .map_err(|e| sync_error("Google refresh token request failed", e))?;

        if !response.status().is_success() {
            let status = response.status();
            if let Ok(body) = response.text().await {
                eprintln!("Google refresh token request failed with {status}: {body}");
            }
            return Err(CalendarSyncError {
                message: match status {
                    StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::NOT_FOUND => {
                        "Google Calendar is not connected. Log out and sign in with calendar permission."
                    }
                    _ => "Could not refresh Google Calendar access.",
                }
                .to_string(),
            });
        }

        let token = response
            .json::<GoogleRefreshResponse>()
            .await
            .map_err(|e| sync_error("Google refresh token response was invalid", e))?;

        if token.access_token.trim().is_empty() {
            return Err(CalendarSyncError {
                message: "Google did not return a Calendar access token.".to_string(),
            });
        }

        Ok(token.access_token)
    }
}

#[derive(Deserialize)]
struct GoogleRefreshResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct GoogleEventResponse {
    id: String,
    #[serde(rename = "htmlLink")]
    html_link: Option<String>,
}

#[derive(Deserialize)]
struct GoogleErrorResponse {
    error: Option<GoogleErrorBody>,
}

#[derive(Deserialize)]
struct GoogleErrorBody {
    message: Option<String>,
}

fn calendar_event_body(todo: &CalendarTodo<'_>) -> serde_json::Value {
    let status = if todo.completed {
        "completed"
    } else {
        "active"
    };
    let summary = if todo.completed {
        format!("Done: {}", todo.title)
    } else {
        format!("Todo: {}", todo.title)
    };
    let (start, end) = calendar_event_time(todo);

    json!({
        "summary": summary,
        "description": format!("Synced from Glum todo. Status: {status}."),
        "start": start,
        "end": end,
        "transparency": "transparent",
        "extendedProperties": {
            "private": {
                "glumTodoId": todo.id.to_string(),
                "glumTodoStatus": status
            }
        }
    })
}

fn calendar_event_time(todo: &CalendarTodo<'_>) -> (serde_json::Value, serde_json::Value) {
    if todo.due_has_time {
        return (
            json!({ "dateTime": todo_calendar_datetime(todo.due_date) }),
            json!({ "dateTime": todo_calendar_datetime(todo.due_date + Duration::hours(1)) }),
        );
    }

    (
        json!({ "date": todo_calendar_date(todo.due_date) }),
        json!({ "date": todo_calendar_date(todo.due_date + Duration::days(1)) }),
    )
}

fn todo_calendar_date(date: OffsetDateTime) -> String {
    // The UI stores date-only todos as a timestamp. Adding noon keeps older
    // midnight-local rows from landing on the previous UTC date for most users.
    let date = (date + Duration::hours(12)).date();
    format!(
        "{:04}-{:02}-{:02}",
        date.year(),
        u8::from(date.month()),
        date.day()
    )
}

fn todo_calendar_datetime(date: OffsetDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        date.year(),
        u8::from(date.month()),
        date.day(),
        date.hour(),
        date.minute(),
        date.second()
    )
}

async fn google_error_message(status: StatusCode, response: reqwest::Response) -> String {
    if let Ok(error) = response.json::<GoogleErrorResponse>().await {
        if let Some(message) = error.error.and_then(|error| error.message) {
            if !message.trim().is_empty() {
                return message;
            }
        }
    }

    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            "Google Calendar rejected the request. Re-login and grant calendar access."
        }
        StatusCode::NOT_FOUND => "The Google Calendar event was not found.",
        _ => "Google Calendar sync failed.",
    }
    .to_string()
}

fn sync_error(operation: &str, error: reqwest::Error) -> CalendarSyncError {
    eprintln!("{operation}: {error:?}");
    CalendarSyncError {
        message: "Google Calendar sync failed.".to_string(),
    }
}
