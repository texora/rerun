#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use criterion::{criterion_group, criterion_main, Criterion};

use itertools::Itertools;
use re_data_store::{DataStore, LatestAtQuery};
use re_log_types::{entity_path, DataRow, EntityPath, RowId, TimeInt, TimeType, Timeline};
use re_query2::{clamped_zip_1x1, LatestAtResults, PromiseResolver};
use re_types::{
    archetypes::Points2D,
    components::{Color, InstanceKey, Position2D, Text},
    Archetype as _,
};
use re_types_core::Loggable as _;

// ---

#[cfg(not(debug_assertions))]
const NUM_FRAMES_POINTS: u32 = 1_000;
#[cfg(not(debug_assertions))]
const NUM_POINTS: u32 = 1_000;
#[cfg(not(debug_assertions))]
const NUM_FRAMES_STRINGS: u32 = 1_000;
#[cfg(not(debug_assertions))]
const NUM_STRINGS: u32 = 1_000;

// `cargo test` also runs the benchmark setup code, so make sure they run quickly:
#[cfg(debug_assertions)]
const NUM_FRAMES_POINTS: u32 = 1;
#[cfg(debug_assertions)]
const NUM_POINTS: u32 = 1;
#[cfg(debug_assertions)]
const NUM_FRAMES_STRINGS: u32 = 1;
#[cfg(debug_assertions)]
const NUM_STRINGS: u32 = 1;

criterion_group!(
    benches,
    mono_points,
    mono_strings,
    batch_points,
    batch_strings
);
criterion_main!(benches);

// --- Benchmarks ---

pub fn build_some_point2d(len: usize) -> Vec<Position2D> {
    use rand::Rng as _;
    let mut rng = rand::thread_rng();

    (0..len)
        .map(|_| Position2D::new(rng.gen_range(0.0..10.0), rng.gen_range(0.0..10.0)))
        .collect()
}

/// Create `len` dummy colors
pub fn build_some_colors(len: usize) -> Vec<Color> {
    (0..len).map(|i| Color::from(i as u32)).collect()
}

/// Build a ([`Timeline`], [`TimeInt`]) tuple from `frame_nr` suitable for inserting in a [`re_log_types::TimePoint`].
pub fn build_frame_nr(frame_nr: TimeInt) -> (Timeline, TimeInt) {
    (Timeline::new("frame_nr", TimeType::Sequence), frame_nr)
}

pub fn build_some_strings(len: usize) -> Vec<Text> {
    use rand::Rng as _;
    let mut rng = rand::thread_rng();

    (0..len)
        .map(|_| {
            let ilen: usize = rng.gen_range(0..10000);
            let s: String = rand::thread_rng()
                .sample_iter(&rand::distributions::Alphanumeric)
                .take(ilen)
                .map(char::from)
                .collect();
            Text::from(s)
        })
        .collect()
}

fn mono_points(c: &mut Criterion) {
    // Each mono point gets logged at a different path
    let paths = (0..NUM_POINTS)
        .map(move |point_idx| entity_path!("points", point_idx.to_string()))
        .collect_vec();
    let msgs = build_points_rows(&paths, 1);

    {
        let mut group = c.benchmark_group("arrow_mono_points2");
        // Mono-insert is slow -- decrease the sample size
        group.sample_size(10);
        group.throughput(criterion::Throughput::Elements(
            (NUM_POINTS * NUM_FRAMES_POINTS) as _,
        ));
        group.bench_function("insert", |b| {
            b.iter(|| insert_rows(msgs.iter()));
        });
    }

    {
        let mut group = c.benchmark_group("arrow_mono_points2");
        group.throughput(criterion::Throughput::Elements(NUM_POINTS as _));
        let store = insert_rows(msgs.iter());
        group.bench_function("query", |b| {
            b.iter(|| query_and_visit_points(&store, &paths));
        });
    }
}

fn mono_strings(c: &mut Criterion) {
    // Each mono string gets logged at a different path
    let paths = (0..NUM_STRINGS)
        .map(move |string_idx| entity_path!("strings", string_idx.to_string()))
        .collect_vec();
    let msgs = build_strings_rows(&paths, 1);

    {
        let mut group = c.benchmark_group("arrow_mono_strings2");
        group.sample_size(10);
        group.throughput(criterion::Throughput::Elements(
            (NUM_STRINGS * NUM_FRAMES_STRINGS) as _,
        ));
        group.bench_function("insert", |b| {
            b.iter(|| insert_rows(msgs.iter()));
        });
    }

    {
        let mut group = c.benchmark_group("arrow_mono_strings2");
        group.throughput(criterion::Throughput::Elements(NUM_POINTS as _));
        let store = insert_rows(msgs.iter());
        group.bench_function("query", |b| {
            b.iter(|| query_and_visit_strings(&store, &paths));
        });
    }
}

fn batch_points(c: &mut Criterion) {
    // Batch points are logged together at a single path
    let paths = [EntityPath::from("points")];
    let msgs = build_points_rows(&paths, NUM_POINTS as _);

    {
        let mut group = c.benchmark_group("arrow_batch_points2");
        group.throughput(criterion::Throughput::Elements(
            (NUM_POINTS * NUM_FRAMES_POINTS) as _,
        ));
        group.bench_function("insert", |b| {
            b.iter(|| insert_rows(msgs.iter()));
        });
    }

    {
        let mut group = c.benchmark_group("arrow_batch_points2");
        group.throughput(criterion::Throughput::Elements(NUM_POINTS as _));
        let store = insert_rows(msgs.iter());
        group.bench_function("query", |b| {
            b.iter(|| query_and_visit_points(&store, &paths));
        });
    }
}

fn batch_strings(c: &mut Criterion) {
    // Batch strings are logged together at a single path
    let paths = [EntityPath::from("points")];
    let msgs = build_strings_rows(&paths, NUM_STRINGS as _);

    {
        let mut group = c.benchmark_group("arrow_batch_strings2");
        group.throughput(criterion::Throughput::Elements(
            (NUM_STRINGS * NUM_FRAMES_STRINGS) as _,
        ));
        group.bench_function("insert", |b| {
            b.iter(|| insert_rows(msgs.iter()));
        });
    }

    {
        let mut group = c.benchmark_group("arrow_batch_strings2");
        group.throughput(criterion::Throughput::Elements(NUM_POINTS as _));
        let store = insert_rows(msgs.iter());
        group.bench_function("query", |b| {
            b.iter(|| query_and_visit_strings(&store, &paths));
        });
    }
}

// --- Helpers ---

fn build_points_rows(paths: &[EntityPath], num_points: usize) -> Vec<DataRow> {
    (0..NUM_FRAMES_POINTS)
        .flat_map(move |frame_idx| {
            paths.iter().map(move |path| {
                let mut row = DataRow::from_cells2(
                    RowId::new(),
                    path.clone(),
                    [build_frame_nr((frame_idx as i64).try_into().unwrap())],
                    num_points as _,
                    (
                        build_some_point2d(num_points),
                        build_some_colors(num_points),
                    ),
                )
                .unwrap();
                // NOTE: Using unsized cells will crash in debug mode, and benchmarks are run for 1 iteration,
                // in debug mode, by the standard test harness.
                if cfg!(debug_assertions) {
                    row.compute_all_size_bytes();
                }
                row
            })
        })
        .collect()
}

fn build_strings_rows(paths: &[EntityPath], num_strings: usize) -> Vec<DataRow> {
    (0..NUM_FRAMES_STRINGS)
        .flat_map(move |frame_idx| {
            paths.iter().map(move |path| {
                let mut row = DataRow::from_cells2(
                    RowId::new(),
                    path.clone(),
                    [build_frame_nr((frame_idx as i64).try_into().unwrap())],
                    num_strings as _,
                    // We still need to create points because they are the primary for the
                    // archetype query we want to do. We won't actually deserialize the points
                    // during the query -- we just need it for the primary keys.
                    // TODO(jleibs): switch this to use `TextEntry` once the new type has
                    // landed.
                    (
                        build_some_point2d(num_strings),
                        build_some_strings(num_strings),
                    ),
                )
                .unwrap();
                // NOTE: Using unsized cells will crash in debug mode, and benchmarks are run for 1 iteration,
                // in debug mode, by the standard test harness.
                if cfg!(debug_assertions) {
                    row.compute_all_size_bytes();
                }
                row
            })
        })
        .collect()
}

fn insert_rows<'a>(msgs: impl Iterator<Item = &'a DataRow>) -> DataStore {
    let mut store = DataStore::new(
        re_log_types::StoreId::random(re_log_types::StoreKind::Recording),
        InstanceKey::name(),
        Default::default(),
    );
    msgs.for_each(|row| {
        store.insert_row(row).unwrap();
    });
    store
}

struct SavePoint {
    _pos: Position2D,
    _color: Option<Color>,
}

fn query_and_visit_points(store: &DataStore, paths: &[EntityPath]) -> Vec<SavePoint> {
    let resolver = PromiseResolver::default();

    let timeline_frame_nr = Timeline::new("frame_nr", TimeType::Sequence);
    let query = LatestAtQuery::new(timeline_frame_nr, NUM_FRAMES_POINTS as i64 / 2);

    let mut ret = Vec::with_capacity(NUM_POINTS as _);

    // TODO(jleibs): Add Radius once we have support for it in field_types
    for entity_path in paths {
        let results: LatestAtResults = re_query2::latest_at(
            store,
            &query,
            entity_path,
            Points2D::all_components().iter().cloned(), // no generics!
        );

        let points = results.get_required(Position2D::name()).unwrap();
        let colors = results.get_or_empty(Color::name());

        let points = points
            .iter_dense::<Position2D>(&resolver)
            .flatten()
            .unwrap();

        let colors = colors.iter_dense::<Color>(&resolver).flatten().unwrap();
        let color_default_fn = || Color::from(0xFF00FFFF);

        for (point, color) in clamped_zip_1x1(points, colors, color_default_fn) {
            ret.push(SavePoint {
                _pos: point,
                _color: Some(color),
            });
        }
    }
    assert_eq!(NUM_POINTS as usize, ret.len());
    ret
}

struct SaveString {
    _label: Option<Text>,
}

fn query_and_visit_strings(store: &DataStore, paths: &[EntityPath]) -> Vec<SaveString> {
    let resolver = PromiseResolver::default();

    let timeline_frame_nr = Timeline::new("frame_nr", TimeType::Sequence);
    let query = LatestAtQuery::new(timeline_frame_nr, NUM_FRAMES_STRINGS as i64 / 2);

    let mut strings = Vec::with_capacity(NUM_STRINGS as _);

    for entity_path in paths {
        let results: LatestAtResults = re_query2::latest_at(
            store,
            &query,
            entity_path,
            Points2D::all_components().iter().cloned(), // no generics!
        );

        let points = results.get_required(Position2D::name()).unwrap();
        let colors = results.get_or_empty(Text::name());

        let points = points
            .iter_dense::<Position2D>(&resolver)
            .flatten()
            .unwrap();

        let labels = colors.iter_dense::<Text>(&resolver).flatten().unwrap();
        let label_default_fn = || Text(String::new().into());

        for (_point, label) in clamped_zip_1x1(points, labels, label_default_fn) {
            strings.push(SaveString {
                _label: Some(label),
            });
        }
    }
    assert_eq!(NUM_STRINGS as usize, strings.len());

    criterion::black_box(strings)
}
