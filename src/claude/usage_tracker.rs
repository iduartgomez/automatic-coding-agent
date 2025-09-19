use crate::claude::types::{SessionId, TaskResponse, TokenUsage, UsageTrackingConfig};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct UsageTracker {
    config: UsageTrackingConfig,
    usage_data: Arc<Mutex<UsageData>>,
}

#[derive(Debug, Default)]
struct UsageData {
    sessions: HashMap<SessionId, SessionUsage>,
    daily_usage: HashMap<String, DailyUsage>, // Date string -> usage
    total_usage: TotalUsage,
}

#[derive(Debug, Clone)]
pub struct SessionUsage {
    pub session_id: SessionId,
    pub start_time: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub token_usage: TokenUsage,
    pub request_count: u32,
    pub total_cost: f64,
    pub average_response_time: Duration,
}

#[derive(Debug, Clone, Default)]
pub struct DailyUsage {
    pub date: String,
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub request_count: u32,
    pub total_cost: f64,
    pub unique_sessions: u32,
}

#[derive(Debug, Clone, Default)]
pub struct TotalUsage {
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_requests: u64,
    pub total_cost: f64,
    pub total_sessions: u64,
    pub first_request: Option<DateTime<Utc>>,
    pub last_request: Option<DateTime<Utc>>,
}

impl UsageTracker {
    pub fn new(config: UsageTrackingConfig) -> Self {
        Self {
            config,
            usage_data: Arc::new(Mutex::new(UsageData::default())),
        }
    }

    pub async fn start_session(&self, session_id: SessionId) {
        if !self.config.track_tokens && !self.config.track_costs && !self.config.track_performance {
            return;
        }

        let mut data = self.usage_data.lock().await;

        data.sessions.insert(session_id, SessionUsage {
            session_id,
            start_time: Utc::now(),
            last_activity: Utc::now(),
            token_usage: TokenUsage {
                input_tokens: 0,
                output_tokens: 0,
                total_tokens: 0,
                estimated_cost: 0.0,
            },
            request_count: 0,
            total_cost: 0.0,
            average_response_time: Duration::from_millis(0),
        });

        data.total_usage.total_sessions += 1;
    }

    pub async fn record_usage(&self, session_id: SessionId, response: &TaskResponse) {
        if !self.config.track_tokens && !self.config.track_costs && !self.config.track_performance {
            return;
        }

        let mut data = self.usage_data.lock().await;
        let now = Utc::now();

        // Update session usage
        if let Some(session) = data.sessions.get_mut(&session_id) {
            session.last_activity = now;
            session.request_count += 1;

            if self.config.track_tokens {
                session.token_usage.input_tokens += response.token_usage.input_tokens;
                session.token_usage.output_tokens += response.token_usage.output_tokens;
                session.token_usage.total_tokens += response.token_usage.total_tokens;
            }

            if self.config.track_costs {
                session.token_usage.estimated_cost += response.token_usage.estimated_cost;
                session.total_cost += response.token_usage.estimated_cost;
            }

            if self.config.track_performance {
                // Update average response time
                let total_time = session.average_response_time.as_millis() as u64 * (session.request_count - 1) as u64
                    + response.execution_time.as_millis() as u64;
                session.average_response_time = Duration::from_millis(total_time / session.request_count as u64);
            }
        }

        // Update daily usage
        let date_key = now.format("%Y-%m-%d").to_string();
        let daily = data.daily_usage.entry(date_key.clone()).or_insert_with(|| DailyUsage {
            date: date_key,
            ..Default::default()
        });

        daily.request_count += 1;
        if self.config.track_tokens {
            daily.total_tokens += response.token_usage.total_tokens;
            daily.input_tokens += response.token_usage.input_tokens;
            daily.output_tokens += response.token_usage.output_tokens;
        }

        if self.config.track_costs {
            daily.total_cost += response.token_usage.estimated_cost;
        }

        // Update total usage
        data.total_usage.total_requests += 1;
        if self.config.track_tokens {
            data.total_usage.total_tokens += response.token_usage.total_tokens;
            data.total_usage.input_tokens += response.token_usage.input_tokens;
            data.total_usage.output_tokens += response.token_usage.output_tokens;
        }

        if self.config.track_costs {
            data.total_usage.total_cost += response.token_usage.estimated_cost;
        }

        if data.total_usage.first_request.is_none() {
            data.total_usage.first_request = Some(now);
        }
        data.total_usage.last_request = Some(now);

        // Clean up old data based on retention policy
        self.cleanup_old_data(&mut data, now).await;
    }

    async fn cleanup_old_data(&self, data: &mut UsageData, now: DateTime<Utc>) {
        let cutoff = now - self.config.history_retention;

        // Remove old sessions
        data.sessions.retain(|_, session| session.last_activity > cutoff);

        // Remove old daily usage
        data.daily_usage.retain(|date_str, _| {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                let datetime = date.and_time(chrono::NaiveTime::MIN).and_utc();
                datetime > cutoff
            } else {
                false
            }
        });
    }

    pub async fn get_session_usage(&self, session_id: SessionId) -> Option<SessionUsage> {
        let data = self.usage_data.lock().await;
        data.sessions.get(&session_id).cloned()
    }

    pub async fn get_daily_usage(&self, date: &str) -> Option<DailyUsage> {
        let data = self.usage_data.lock().await;
        data.daily_usage.get(date).cloned()
    }

    pub async fn get_total_usage(&self) -> TotalUsage {
        let data = self.usage_data.lock().await;
        data.total_usage.clone()
    }

    pub async fn get_usage_summary(&self, days: u32) -> UsageSummary {
        let data = self.usage_data.lock().await;
        let now = Utc::now();
        let cutoff = now - chrono::Duration::days(days as i64);

        let mut summary = UsageSummary {
            period_days: days,
            total_requests: 0,
            total_tokens: 0,
            total_cost: 0.0,
            unique_sessions: 0,
            average_tokens_per_request: 0.0,
            average_cost_per_request: 0.0,
            daily_breakdown: Vec::new(),
        };

        // Collect daily usage within the period
        for (_, daily) in data.daily_usage.iter() {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(&daily.date, "%Y-%m-%d") {
                let datetime = date.and_time(chrono::NaiveTime::MIN).and_utc();
                if datetime >= cutoff {
                    summary.total_requests += daily.request_count;
                    summary.total_tokens += daily.total_tokens;
                    summary.total_cost += daily.total_cost;
                    summary.daily_breakdown.push(daily.clone());
                }
            }
        }

        // Count unique sessions in the period
        summary.unique_sessions = data.sessions.values()
            .filter(|session| session.last_activity >= cutoff)
            .count() as u32;

        // Calculate averages
        if summary.total_requests > 0 {
            summary.average_tokens_per_request = summary.total_tokens as f64 / summary.total_requests as f64;
            summary.average_cost_per_request = summary.total_cost / summary.total_requests as f64;
        }

        // Sort daily breakdown by date
        summary.daily_breakdown.sort_by(|a, b| a.date.cmp(&b.date));

        summary
    }

    pub async fn estimate_cost_for_tokens(&self, input_tokens: u64, output_tokens: u64) -> f64 {
        // Simple cost estimation - in practice this would be model-specific
        const INPUT_COST_PER_TOKEN: f64 = 0.000003; // $3 per million tokens
        const OUTPUT_COST_PER_TOKEN: f64 = 0.000015; // $15 per million tokens

        (input_tokens as f64 * INPUT_COST_PER_TOKEN) + (output_tokens as f64 * OUTPUT_COST_PER_TOKEN)
    }
}

#[derive(Debug, Clone)]
pub struct UsageSummary {
    pub period_days: u32,
    pub total_requests: u32,
    pub total_tokens: u64,
    pub total_cost: f64,
    pub unique_sessions: u32,
    pub average_tokens_per_request: f64,
    pub average_cost_per_request: f64,
    pub daily_breakdown: Vec<DailyUsage>,
}