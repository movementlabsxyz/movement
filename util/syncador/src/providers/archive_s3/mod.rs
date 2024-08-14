/// let push = PushPipeline(vec![
/// MatcherPush::new(),
/// ArchiveGzipPush::new(),
/// S3Push::new(),
/// ]);
///
/// let pull = PullPipeline(vec![
/// S3Pull::new(),
/// ArchiveGzipPull::new(),
/// ]);
///
/// let push_runner = Every(10, minutes).push(push);
/// let pull_runner = Once.pull(pull);
///
/// let syncer = Syncer::new(push_runner, pull_runner);
/// syncer.run().await?;
pub mod nothing_yet {}
