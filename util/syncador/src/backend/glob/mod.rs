use regex::Regex;

#[derive(Debug, Clone)]
pub struct Glob {
	pub pattern: Regex,
}
