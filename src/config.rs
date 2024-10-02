use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum GameState {
    Configuring,
    Started,
    Ended,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Answer {
    SingleAnswer(String),
    MutlipleAnswer(Vec<String>, #[serde(skip)] usize),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Question {
    pub name: String,
    pub answer: Vec<Answer>,
}

impl Question {
    pub fn normalize_string(s: &str) -> String {
        s.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase()
    }

    pub fn is_answer(&self, s: &str) -> bool {
        self.get_answer_pos(s).is_some()
    }

    pub fn get_answer_pos(&self, s: &str) -> Option<(usize, f64)> {
        let words = Self::normalize_string(s);
        let pos = self.answer.iter().position(|a| match a {
            Answer::SingleAnswer(astr) => *astr == words,
            Answer::MutlipleAnswer(astrs, _) => astrs.iter().any(|s| *s == words),
        });
        pos.map(|p| {
            (
                p,
                match self.answer.get(p).unwrap() {
                    Answer::SingleAnswer(_) => 1.0,
                    Answer::MutlipleAnswer(astrs, len) => {
                        if astrs.len() == *len {
                            1.0
                        } else {
                            0.5
                        }
                    }
                },
            )
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuildConfig {
    pub teams: Vec<Team>,
    pub admin_channel: serenity::all::ChannelId,
    pub state: GameState,
    pub questions: Vec<Question>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Team {
    pub name: String,
    pub leaderboard: HashMap<serenity::all::UserId, f64>,
    pub total_points: f64,
    pub channel: serenity::all::ChannelId,
}
