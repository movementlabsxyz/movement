pub mod backend;
pub mod godfig;
pub use godfig::*;

#[macro_export]
macro_rules! env_default {
	// Case with default value
	($name:ident, $env:expr, $ty:ty, $default:expr) => {
		pub fn $name() -> $ty {
			std::env::var($env).ok().and_then(|v| v.parse().ok()).unwrap_or($default)
		}
	};
	// Case without default value
	($name:ident, $env:expr, $ty:ty) => {
		pub fn $name() -> Option<$ty> {
			std::env::var($env).ok().and_then(|v| v.parse().ok())
		}
	};
}

#[macro_export]
macro_rules! env_short_default {
	// Case with default value
	($name:ident, $ty:ty, $default:expr) => {
		pub fn $name() -> $ty {
			std::env::var(stringify!($name).to_uppercase())
				.ok()
				.and_then(|v| v.parse::<$ty>().ok())
				.unwrap_or_else(|| $default.into())
		}
	};
}

#[macro_export]
macro_rules! env_or_none {
    ($fname:ident, $ty:ty, $( $name:ident ),* ) => {
        pub fn $fname() -> Option<$ty> {
            let vars_set = vec![
                $(
                    std::env::var(stringify!($name).to_uppercase()).ok().filter(|v| !v.is_empty())
                ),*
            ];

            if vars_set.iter().all(Option::is_some) {
                Some(<$ty>::default())
            } else {
                None
            }
        }
    };
}

#[cfg(test)]
mod tests {

	#[test]
	fn test_env_default_with_env() {
		std::env::set_var("TEST_ENV_DEFAULT_1", "42");

		// without default value
		env_default!(my_env, "TEST_ENV_DEFAULT_1", i32);
		let result = my_env();
		assert_eq!(result, Some(42));

		// with default value
		env_default!(my_env_with_default, "TEST_ENV_DEFAULT_1", i32, 0);
		let result = my_env_with_default();
		assert_eq!(result, 42);
	}

	#[test]
	fn test_env_default_without_env() {
		std::env::remove_var("TEST_ENV_DEFAULT_2");

		// without default value
		env_default!(my_env, "TEST_ENV_DEFAULT_2", i32);
		let result = my_env();
		assert_eq!(result, None);

		// with default value
		env_default!(my_env_with_default, "TEST_ENV_DEFAULT_2", i32, 0);
		let result = my_env_with_default();
		assert_eq!(result, 0);
	}

	#[test]
	fn test_short_env_or_none_with_env() {
		env_short_default!(my_short_env, i32, 0);
		std::env::set_var("MY_SHORT_ENV", "42");
		env_short_default!(my_short_env_2, i32, 0);
		std::env::set_var("MY_SHORT_ENV_2", "42");

		env_or_none!(needs_envs, String, my_short_env, my_short_env_2);

		let result = my_short_env();
		assert_eq!(result, 42);

		let result = my_short_env_2();
		assert_eq!(result, 42);

		let result = needs_envs();
		assert_eq!(result, Some(String::default()));

		env_short_default!(my_short_env_3, i32, 0);

		let result = my_short_env_3();
		assert_eq!(result, 0);

		env_or_none!(needs_more_envs, String, my_short_env_3, my_short_env_2, my_short_env);

		let result = needs_more_envs();
		assert_eq!(result, None);
	}
}
