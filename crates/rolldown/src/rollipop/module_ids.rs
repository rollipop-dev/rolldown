//! Numeric runtime ids of Rollipop modules.
//!
//! See internal-docs/rollipop-module-ids/design.md

use oxc_index::IndexVec;
use rolldown_common::{ModuleIdx, ModuleTable};
use xxhash_rust::xxh3::xxh3_64;

/// Size of the id space. Ids are `xxh3(stable_id) mod ROLLIPOP_MODULE_ID_SPACE`.
///
/// Fixed, never derived from the module count: scaling the space with the graph would
/// renumber every module whenever the count crosses a threshold. 2^24 keeps ids at 8
/// decimal digits or fewer and inside every engine's small-integer range, while collisions
/// (resolved by probing) stay rare: about `n^2 / 2^25` pairs for `n` modules.
pub const ROLLIPOP_MODULE_ID_SPACE: u32 = 1 << 24;

/// The numeric runtime id of every normal module, assigned once per build.
///
/// The finalizer (`__rollipop_require__(id)`) and the chunk renderer (`id: function(...)`)
/// both read ids from here, so they always agree.
#[derive(Debug, Default)]
pub struct RollipopModuleIds {
  ids: IndexVec<ModuleIdx, Option<u32>>,
}

impl RollipopModuleIds {
  pub fn new(module_table: &ModuleTable) -> Self {
    let modules = module_table
      .modules
      .iter()
      .filter_map(|module| module.as_normal())
      .map(|module| (module.idx, module.stable_id.as_str(), module.id.as_str()));
    Self { ids: assign_module_ids(modules, module_table.modules.len(), ROLLIPOP_MODULE_ID_SPACE) }
  }

  /// The runtime id of a bundled module.
  ///
  /// # Panics
  /// If `idx` is not a normal module of the table the ids were assigned for.
  pub fn get(&self, idx: ModuleIdx) -> u32 {
    self.ids.get(idx).copied().flatten().expect("every normal module has a Rollipop module id")
  }
}

/// Assigns each module `xxh3(stable_id) mod space`, probing linearly on collisions.
///
/// Modules are processed in `(stable_id, id)` order, so the result depends only on the set
/// of modules, never on `ModuleIdx` (loader discovery order). A module's id only changes
/// when a module that collides with it, and sorts before it, is added or removed.
fn assign_module_ids<'a>(
  modules: impl Iterator<Item = (ModuleIdx, &'a str, &'a str)>,
  module_count: usize,
  space: u32,
) -> IndexVec<ModuleIdx, Option<u32>> {
  let mut modules = modules.collect::<Vec<_>>();
  assert!(modules.len() <= space as usize, "too many modules for the Rollipop module id space");
  // `stable_id` is cwd-relative, so it can be shared, e.g. by a plugin's relative id and
  // an absolute path. The full `id` makes the order total.
  modules.sort_unstable_by(|(_, a_stable_id, a_id), (_, b_stable_id, b_id)| {
    a_stable_id.cmp(b_stable_id).then_with(|| a_id.cmp(b_id))
  });

  let mut ids = IndexVec::from_vec(vec![None; module_count]);
  let mut taken = rustc_hash::FxHashSet::default();
  taken.reserve(modules.len());
  for (idx, stable_id, _) in modules {
    #[expect(clippy::cast_possible_truncation, reason = "the remainder is below `space`")]
    let mut id = (xxh3_64(stable_id.as_bytes()) % u64::from(space)) as u32;
    while !taken.insert(id) {
      id = (id + 1) % space;
    }
    ids[idx] = Some(id);
  }
  ids
}

#[cfg(test)]
mod tests {
  use super::*;

  fn assign(modules: &[(u32, &'static str)], space: u32) -> Vec<Option<u32>> {
    let count = modules.iter().map(|(idx, _)| *idx as usize + 1).max().unwrap_or(0);
    assign_module_ids(
      modules.iter().map(|(idx, stable_id)| (ModuleIdx::from_raw(*idx), *stable_id, *stable_id)),
      count,
      space,
    )
    .raw
  }

  fn hash(stable_id: &str, space: u32) -> u32 {
    u32::try_from(xxh3_64(stable_id.as_bytes()) % u64::from(space)).unwrap()
  }

  #[test]
  fn ids_are_hashes_of_stable_ids() {
    let space = ROLLIPOP_MODULE_ID_SPACE;
    let ids = assign(&[(0, "src/a.js"), (1, "src/b.js")], space);
    assert_eq!(ids, vec![Some(hash("src/a.js", space)), Some(hash("src/b.js", space))]);
  }

  #[test]
  fn ids_do_not_depend_on_module_idx() {
    let space = ROLLIPOP_MODULE_ID_SPACE;
    let forward = assign(&[(0, "a.js"), (1, "b.js"), (2, "c.js")], space);
    let backward = assign(&[(2, "a.js"), (1, "b.js"), (0, "c.js")], space);
    assert_eq!(forward, backward.into_iter().rev().collect::<Vec<_>>());
  }

  #[test]
  fn collisions_probe_in_stable_id_order() {
    // A one-slot-per-module space forces collisions.
    let space = 4;
    let names = ["a.js", "b.js", "c.js", "d.js"];
    let ids = assign(&[(0, names[3]), (1, names[2]), (2, names[1]), (3, names[0])], space);
    let mut seen = ids.iter().map(|id| id.unwrap()).collect::<Vec<_>>();
    seen.sort_unstable();
    assert_eq!(seen, vec![0, 1, 2, 3], "every module gets a distinct id");
    // The module sorting first always gets its hash slot.
    assert_eq!(ids[3], Some(hash("a.js", space)));
    // Discovery order doesn't matter.
    let reordered = assign(&[(0, names[0]), (1, names[1]), (2, names[2]), (3, names[3])], space);
    assert_eq!(ids, reordered.into_iter().rev().collect::<Vec<_>>());
  }

  #[test]
  fn probing_wraps_around() {
    let space = 2;
    // Find two names that hash to the last slot.
    let mut names = (0..).map(|i| format!("m{i}.js")).filter(|name| hash(name, space) == 1);
    let first = names.next().unwrap();
    let second = names.next().unwrap();
    let (first, second) = if first < second { (first, second) } else { (second, first) };
    let ids = assign_module_ids(
      [(ModuleIdx::from_raw(0), second.as_str(), ""), (ModuleIdx::from_raw(1), first.as_str(), "")]
        .into_iter(),
      2,
      space,
    );
    assert_eq!(ids.raw, vec![Some(0), Some(1)]);
  }

  #[test]
  fn shared_stable_ids_are_ordered_by_id() {
    let space = 2;
    let ids = assign_module_ids(
      [(ModuleIdx::from_raw(0), "x.js", "/b/x.js"), (ModuleIdx::from_raw(1), "x.js", "/a/x.js")]
        .into_iter(),
      2,
      space,
    );
    let first = hash("x.js", space);
    assert_eq!(ids.raw, vec![Some((first + 1) % space), Some(first)]);
  }

  #[test]
  fn adding_a_module_keeps_other_ids() {
    let space = ROLLIPOP_MODULE_ID_SPACE;
    let before = assign(&[(0, "src/main.js"), (1, "src/a.js"), (2, "src/b.js")], space);
    let after =
      assign(&[(0, "src/main.js"), (1, "src/new.js"), (2, "src/a.js"), (3, "src/b.js")], space);
    assert_eq!(before[0], after[0]);
    assert_eq!(before[1], after[2]);
    assert_eq!(before[2], after[3]);
  }
}
