use crate::{cryptography, Signing};
use std::error;
use std::future::Future;

pub trait ToCanonicalString {
	fn to_canonical_string(&self) -> String;
}

pub trait TryFromCanonicalString: Sized {
	fn try_from_canonical_string(s: &str) -> Result<Self, String>;
}

#[derive(Debug)]
pub enum Organization {
	Movement,
	Other(String),
}

impl ToCanonicalString for Organization {
	fn to_canonical_string(&self) -> String {
		match self {
			Organization::Movement => "movement".to_string(),
			Organization::Other(s) => s.clone(),
		}
	}
}

impl TryFromCanonicalString for Organization {
	fn try_from_canonical_string(s: &str) -> Result<Self, String> {
		match s {
			"movement" => Ok(Organization::Movement),
			_ => Ok(Organization::Other(s.to_string())),
		}
	}
}

#[derive(Debug)]
pub enum Environment {
	Prod,
	Dev,
	Staging,
}

impl ToCanonicalString for Environment {
	fn to_canonical_string(&self) -> String {
		match self {
			Environment::Prod => "prod".to_string(),
			Environment::Dev => "dev".to_string(),
			Environment::Staging => "staging".to_string(),
		}
	}
}

impl TryFromCanonicalString for Environment {
	fn try_from_canonical_string(s: &str) -> Result<Self, String> {
		match s {
			"prod" => Ok(Environment::Prod),
			"dev" => Ok(Environment::Dev),
			"staging" => Ok(Environment::Staging),
			_ => Err(format!("invalid environment: {}", s)),
		}
	}
}

#[derive(Debug)]
pub enum SoftwareUnit {
	FullNode,
	Other(String),
}

impl ToCanonicalString for SoftwareUnit {
	fn to_canonical_string(&self) -> String {
		match self {
			SoftwareUnit::FullNode => "full_node".to_string(),
			SoftwareUnit::Other(s) => s.clone(),
		}
	}
}

impl TryFromCanonicalString for SoftwareUnit {
	fn try_from_canonical_string(s: &str) -> Result<Self, String> {
		match s {
			"full_node" => Ok(SoftwareUnit::FullNode),
			_ => Ok(SoftwareUnit::Other(s.to_string())),
		}
	}
}

#[derive(Debug)]
pub enum Usage {
	McrSettlement,
	Other(String),
}

impl ToCanonicalString for Usage {
	fn to_canonical_string(&self) -> String {
		match self {
			Usage::McrSettlement => "mcr_settlement".to_string(),
			Usage::Other(s) => s.clone(),
		}
	}
}

impl TryFromCanonicalString for Usage {
	fn try_from_canonical_string(s: &str) -> Result<Self, String> {
		match s {
			"mcr_settlement" => Ok(Usage::McrSettlement),
			_ => Ok(Usage::Other(s.to_string())),
		}
	}
}

#[derive(Debug)]
pub enum AllowedRoles {
	Signer,
	Auditor,
	Other(String),
}

impl ToCanonicalString for AllowedRoles {
	fn to_canonical_string(&self) -> String {
		match self {
			AllowedRoles::Signer => "signer".to_string(),
			AllowedRoles::Auditor => "auditor".to_string(),
			AllowedRoles::Other(s) => s.clone(),
		}
	}
}

impl TryFromCanonicalString for AllowedRoles {
	fn try_from_canonical_string(s: &str) -> Result<Self, String> {
		match s {
			"signer" => Ok(AllowedRoles::Signer),
			"auditor" => Ok(AllowedRoles::Auditor),
			_ => Ok(AllowedRoles::Other(s.to_string())),
		}
	}
}

#[derive(Debug)]
pub struct Key {
	org: Organization,
	environment: Environment,
	software_unit: SoftwareUnit,
	usage: Usage,
	allowed_roles: AllowedRoles,
	key_name: String,
	app_replica: Option<String>,
}

impl Key {
	pub fn new(
		org: Organization,
		environment: Environment,
		software_unit: SoftwareUnit,
		usage: Usage,
		allowed_roles: AllowedRoles,
		key_name: String,
		app_replica: Option<String>,
	) -> Self {
		Self { org, environment, software_unit, usage, allowed_roles, key_name, app_replica }
	}

	pub fn org(&self) -> &Organization {
		&self.org
	}

	pub fn environment(&self) -> &Environment {
		&self.environment
	}

	pub fn software_unit(&self) -> &SoftwareUnit {
		&self.software_unit
	}

	pub fn usage(&self) -> &Usage {
		&self.usage
	}

	pub fn allowed_roles(&self) -> &AllowedRoles {
		&self.allowed_roles
	}

	pub fn key_name(&self) -> &str {
		&self.key_name
	}

	pub fn app_replica(&self) -> Option<&String> {
		self.app_replica.as_ref()
	}

	/// Return a delimited canonical string representation of the key.
	pub fn to_delimited_canonical_string(&self, delimiter: &str) -> String {
		format!(
			"{}{delimiter}{}{delimiter}{}{delimiter}{}{delimiter}{}{delimiter}{}{delimiter}{}",
			self.org.to_canonical_string(),
			self.environment.to_canonical_string(),
			self.software_unit.to_canonical_string(),
			self.usage.to_canonical_string(),
			self.allowed_roles.to_canonical_string(),
			self.key_name,
			self.app_replica.as_deref().unwrap_or("0"),
			delimiter = delimiter
		)
	}

	/// Gets a key from a canonical string.
	/// Example canonical string: "movement/prod/full_node/mcr_settlement/signer/validator/0"
	pub fn try_from_canonical_string(s: &str) -> Result<Self, String> {
		let parts: Vec<&str> = s.split('/').collect();
		if parts.len() != 7 {
			return Err(format!("invalid key: {}", s));
		}

		Ok(Self {
			org: Organization::try_from_canonical_string(parts[0])?,
			environment: Environment::try_from_canonical_string(parts[1])?,
			software_unit: SoftwareUnit::try_from_canonical_string(parts[2])?,
			usage: Usage::try_from_canonical_string(parts[3])?,
			allowed_roles: AllowedRoles::try_from_canonical_string(parts[4])?,
			key_name: parts[5].to_string(),
			app_replica: Some(parts[6].to_string()),
		})
	}

	/// Gets a key from a canonical string environment variable
	pub fn try_from_env_var(var: &str) -> Result<Self, String> {
		let s = std::env::var(var).map_err(|e| format!("{}: {}", var, e))?;
		Self::try_from_canonical_string(&s)
	}
}

/// Errors thrown by [SignerBuilder].
#[derive(Debug, thiserror::Error)]
pub enum SignerBuilderError {
	#[error("building signer failed")]
	BuildingSigner(#[source] Box<dyn error::Error + Send + Sync>),
	#[error("internal error: {0}")]
	Internal(String),
}

pub trait SignerBuilder<C: cryptography::Curve, S: Signing<C>> {
	/// Get async signer for a key.
	fn build(&self, key: Key) -> impl Future<Output = Result<S, SignerBuilderError>> + Send;
}
