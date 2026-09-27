//! Token usage accounting (Phase 2 / T2.1).
//!
//! A trimmed, non-generic equivalent of the TS `NonNullableUsage`: the four
//! counters that both supported protocols report, plus optional provider
//! metadata. Later phases (T3.5) aggregate these across a request/turn.

use std::ops::{Add, AddAssign};

use serde::{Deserialize, Serialize};

/// Token usage reported by a model response.
///
/// All counters default to zero, so a partial wire payload deserializes
/// cleanly. `cache_read_input_tokens` / `cache_creation_input_tokens` are 0 on
/// providers without prompt caching.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Usage {
    /// Prompt tokens billed at full price.
    #[serde(default)]
    pub input_tokens: u64,
    /// Completion tokens.
    #[serde(default)]
    pub output_tokens: u64,
    /// Prompt tokens served from the provider's cache.
    #[serde(default)]
    pub cache_read_input_tokens: u64,
    /// Prompt tokens written to the provider's cache.
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    /// Logical service tier reported by the provider, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
}

impl Usage {
    /// Every counter zero.
    pub const ZERO: Usage = Usage {
        input_tokens: 0,
        output_tokens: 0,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 0,
        service_tier: None,
    };

    /// Total tokens across every counter.
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens
            + self.output_tokens
            + self.cache_read_input_tokens
            + self.cache_creation_input_tokens
    }

    /// Merge another usage record into this one, keeping the newest non-empty
    /// service tier.
    pub fn merge(&mut self, other: &Usage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.cache_read_input_tokens += other.cache_read_input_tokens;
        self.cache_creation_input_tokens += other.cache_creation_input_tokens;
        if other.service_tier.is_some() {
            self.service_tier = other.service_tier.clone();
        }
    }
}

impl Add for Usage {
    type Output = Usage;

    fn add(mut self, rhs: Usage) -> Usage {
        self.merge(&rhs);
        self
    }
}

impl AddAssign for Usage {
    fn add_assign(&mut self, rhs: Usage) {
        self.merge(&rhs);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_fields_default_to_zero() {
        let usage: Usage = serde_json::from_value(json!({"input_tokens": 10})).unwrap();
        assert_eq!(usage.input_tokens, 10);
        assert_eq!(usage.output_tokens, 0);
        assert_eq!(usage.cache_read_input_tokens, 0);
    }

    #[test]
    fn total_and_merge() {
        let mut a = Usage {
            input_tokens: 1,
            output_tokens: 2,
            ..Usage::ZERO
        };
        let b = Usage {
            input_tokens: 3,
            cache_read_input_tokens: 4,
            service_tier: Some("priority".into()),
            ..Usage::ZERO
        };
        a += b;
        assert_eq!(a.input_tokens, 4);
        assert_eq!(a.output_tokens, 2);
        assert_eq!(a.total_tokens(), 10);
        assert_eq!(a.service_tier.as_deref(), Some("priority"));
    }

    #[test]
    fn zero_const_matches_default() {
        assert_eq!(Usage::ZERO, Usage::default());
    }
}
