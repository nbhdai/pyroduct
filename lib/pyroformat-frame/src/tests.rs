use std::sync::LazyLock;

use compio::fs::File;
use vortex_array::ArrayRef;
use vortex_array::IntoArray;
use vortex_array::VortexSessionExecute;
use vortex_array::array_session;
use vortex_array::arrays::ChunkedArray;
use vortex_array::arrays::StructArray;
use vortex_array::arrays::VarBinArray;
use vortex_array::assert_arrays_eq;
use vortex_array::expr::root;
use vortex_array::expr::select;
use vortex_array::stream::ArrayStreamExt;
use vortex_buffer::ByteBuffer;
use vortex_buffer::ByteBufferMut;
use vortex_buffer::buffer;
use vortex_error::VortexResult;
use vortex_file::OpenOptionsSessionExt;
use vortex_file::WriteOptionsSessionExt;
use vortex_io::session::RuntimeSession;
use vortex_layout::session::LayoutSession;
use vortex_session::VortexSession;

use crate::CompioOpenOptions;
use crate::CpuRuntime;
use crate::EncodeOptions;
use crate::Projection;
use crate::write;
use crate::write_at;

static CPU: LazyLock<CpuRuntime> = LazyLock::new(|| CpuRuntime::new(2));

static SESSION: LazyLock<VortexSession> = LazyLock::new(|| {
    let session = array_session()
        .with::<LayoutSession>()
        .with::<RuntimeSession>();
    vortex_file::register_default_encodings(&session);
    CPU.configure(session)
});

fn options() -> EncodeOptions {
    // Test sessions enable no editions, so they would permit no encodings.
    EncodeOptions::new(SESSION.clone()).disable_editions()
}

fn sample() -> VortexResult<ArrayRef> {
    let strings = ChunkedArray::from_iter([
        VarBinArray::from(vec!["ab", "foo", "bar", "baz"]).into_array(),
        VarBinArray::from(vec!["qux", "quux", "corge", "grault"]).into_array(),
    ])
    .into_array();
    let numbers = ChunkedArray::from_iter([
        buffer![1u32, 2, 3, 4].into_array(),
        buffer![5u32, 6, 7, 8].into_array(),
    ])
    .into_array();
    Ok(StructArray::from_fields(&[("strings", strings), ("numbers", numbers)])?.into_array())
}

#[compio::test]
async fn write_at_then_read_all() -> VortexResult<()> {
    let array = sample()?;
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("all.vortex");

    let file = File::create(&path).await?;
    let summary = write_at(options(), &file, array.to_array_stream()).await?;
    file.close().await?;
    assert_eq!(std::fs::metadata(&path)?.len(), summary.size());

    let file = File::open(&path).await?;
    let opened = CompioOpenOptions::new(SESSION.clone()).open(&file).await?;
    assert_eq!(opened.row_count(), 8);

    let vortex_file = opened.read(&file, &Projection::All).await?;
    let actual = vortex_file.scan()?.into_array_stream()?.read_all().await?;
    assert_arrays_eq!(actual, array, &mut SESSION.create_execution_ctx());
    Ok(())
}

#[compio::test]
async fn sequential_write_matches_positional_write() -> VortexResult<()> {
    let array = sample()?;
    let mut bytes = Vec::new();
    let sequential = write(options(), &mut bytes, array.to_array_stream()).await?;
    assert_eq!(bytes.len() as u64, sequential.size());

    let dir = tempfile::tempdir()?;
    let path = dir.path().join("positional.vortex");
    let file = File::create(&path).await?;
    write_at(options(), &file, array.to_array_stream()).await?;
    file.close().await?;
    assert_eq!(std::fs::read(&path)?, bytes);
    Ok(())
}

#[compio::test]
async fn projected_read_fetches_only_projected_segments() -> VortexResult<()> {
    let array = sample()?;
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("projected.vortex");
    let file = File::create(&path).await?;
    write_at(options(), &file, array.to_array_stream()).await?;
    file.close().await?;

    let file = File::open(&path).await?;
    let opened = CompioOpenOptions::new(SESSION.clone()).open(&file).await?;
    let projection = Projection::field_names(["numbers"]);
    let all = opened.segment_ids(&Projection::All)?;
    let projected = opened.segment_ids(&projection)?;
    assert!(!projected.is_empty());
    assert!(projected.len() < all.len());
    assert!(projected.iter().all(|id| all.contains(id)));

    let vortex_file = opened.read(&file, &projection).await?;
    let numbers = select(["numbers"], root()).bind(vortex_file.dtype())?;
    let actual = vortex_file
        .scan()?
        .with_projection(numbers.clone())
        .into_array_stream()?
        .read_all()
        .await?;

    let expected = vortex_file_in_memory(&path)?
        .scan()?
        .with_projection(numbers)
        .into_array_stream()?
        .read_all()
        .await?;
    assert_arrays_eq!(actual, expected, &mut SESSION.create_execution_ctx());
    Ok(())
}

/// The same file opened through `vortex-file`'s in-memory path, as an independent reference.
fn vortex_file_in_memory(path: &std::path::Path) -> VortexResult<vortex_file::VortexFile> {
    SESSION
        .open_options()
        .open_buffer(ByteBuffer::from(std::fs::read(path)?))
}

#[compio::test]
async fn matches_and_reads_vortex_file_writer() -> VortexResult<()> {
    let array = sample()?;
    let mut bytes = ByteBufferMut::empty();
    SESSION
        .write_options()
        .disable_editions()
        .write(&mut bytes, array.to_array_stream())
        .await?;

    let mut ours = Vec::new();
    write(options(), &mut ours, array.to_array_stream()).await?;
    assert_eq!(
        ours,
        bytes.as_slice(),
        "the encoder matches vortex-file's writer byte for byte"
    );

    let dir = tempfile::tempdir()?;
    let path = dir.path().join("reference.vortex");
    std::fs::write(&path, bytes.as_slice())?;

    let file = File::open(&path).await?;
    let opened = CompioOpenOptions::new(SESSION.clone()).open(&file).await?;
    let vortex_file = opened.read(&file, &Projection::All).await?;
    let actual = vortex_file.scan()?.into_array_stream()?.read_all().await?;
    assert_arrays_eq!(actual, array, &mut SESSION.create_execution_ctx());
    Ok(())
}
