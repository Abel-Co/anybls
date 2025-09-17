// 高性能路由系统
pub mod cache;
pub mod matchers;
pub mod router;
pub mod rule_sets;

pub use cache::{CacheKey, MatchCache};
pub use matchers::{DomainMatcher, IpMatcher, MatcherResult};
pub use router::{HighPerformanceRouter, RouteRule};
pub use rule_sets::{DomainRuleSet, IpRuleSet, RuleSet};
