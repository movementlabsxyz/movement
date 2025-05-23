use maptos_framework_release_util::commit_hash_with_script;

// Example usage of the macro to generate a build script for Elsa.
// change
commit_hash_with_script!(
	BiarritzRc1,                                         // Struct name
	"https://github.com/movementlabsxyz/aptos-core.git", // Repository URL
	"edafe2e5ed6ce462fa81d08faf5d5008fa836ca2",          // Commit hash
	6,                                                   // Bytecode version
	"pre-l1-merge.mrb",                                  // MRB file name
	"CACHE_PRE_L1_MERGE_FRAMEWORK_RELEASE"               // Cache environment variable
);
