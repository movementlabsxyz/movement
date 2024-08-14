use glob::Pattern;

#[derive(Debug, Clone)]
pub struct Glob {
	pub pattern: Pattern,
}
