use maptos_framework_release_util::commit_hash_with_script;

commit_hash_with_script!(
	PostL1Merge,                                         // Struct name
	"https://github.com/movementlabsxyz/aptos-core.git", // Repository URL
	"867b1828618ad33bfb3b10c50665cb67113f60e2",          // Commit hash
	6,                                                   // Bytecode version
	"post-l1-merge.mrb",                                 // MRB file name
	"CACHE_POST_L1_MERGE_FRAMEWORK_RELEASE" // Cache environment variable for the post-l1-merge framework release
);
