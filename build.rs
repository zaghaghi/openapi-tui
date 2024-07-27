use anyhow::Result;
use vergen_git2::{BuildBuilder, Emitter, Git2Builder};

pub fn main() -> Result<()> {
  Emitter::default().add_instructions(&BuildBuilder::all_build()?)?.add_instructions(&Git2Builder::all_git()?)?.emit()
}
