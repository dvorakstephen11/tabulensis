From what you attached, the fastest way to move the **e2e** needle is to attack **string interning during workbook parse**.

### Why this matters for your e2e suite

Your e2e fixtures include very large *dense* sheets. For example, **P5** is configured as **50,000 rows x 100 cols** in dense mode. 

In the fixture generator, “dense” mode writes each cell as a **unique string** like `R{r}C{c}`. 
That implies **millions of distinct strings** in P5.

Right now, `StringPool::intern` stores each new string **twice**: once in `strings: Vec<String>` and again as the owned key in `index: FxHashMap<String, StringId>`. Concretely, it does `owned.clone()` for the Vec and then inserts `owned` into the HashMap. 

On workloads dominated by unique strings (exactly what your dense fixtures generate), that means:

* **2 heap allocations per unique string** (the `to_owned()` + the clone)
* roughly **2x string payload memory** (plus HashMap key overhead)
* lots of allocator churn, which directly hurts **parse_time_ms** and **peak_memory_bytes** in e2e runs

That’s also consistent with how your e2e harness asserts parse time is non-zero for these fixtures and treats parse as a first-class part of total time. 

---

## The improvement: store each string once

Keep `strings: Vec<String>` as the single owner of string bytes, but replace the HashMap key with a **64-bit hash**, and then validate equality against the stored string(s). This removes the second owned copy entirely.

Key idea (interner principle):

> You want a fast “have we seen this exact sequence of bytes before?” index, but you don’t want that index to *own another copy* of the bytes.

### Practical design (fast + minimal blast radius)

* `strings: Vec<String>` stays the same (so `pool.strings()` remains `&[String]` and sinks don’t change).
* `index` becomes `FxHashMap<u64, Bucket>` where `Bucket` is either:

  * a single `StringId` (the normal case), or
  * a small `Vec<StringId>` (only used if hashes collide)

This makes collisions correct (because you still compare actual strings), but in practice you’ll almost never allocate the collision Vec.

---

## Patch

### Replace this file

`core/src/string_pool.rs` (current) 

```rust
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StringId(pub u32);

impl std::fmt::Display for StringId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Default)]
pub struct StringPool {
    strings: Vec<String>,
    index: FxHashMap<String, StringId>,
}

impl StringPool {
    pub fn new() -> Self {
        let mut pool = Self::default();
        pool.intern("");
        pool
    }

    pub fn intern(&mut self, s: &str) -> StringId {
        if let Some(&id) = self.index.get(s) {
            return id;
        }

        let id = StringId(self.strings.len() as u32);
        let owned = s.to_owned();
        self.strings.push(owned.clone());
        self.index.insert(owned, id);
        id
    }

    pub fn resolve(&self, id: StringId) -> &str {
        &self.strings[id.0 as usize]
    }

    pub fn strings(&self) -> &[String] {
        &self.strings
    }

    pub fn into_strings(self) -> Vec<String> {
        self.strings
    }

    pub fn len(&self) -> usize {
        self.strings.len()
    }

    pub fn estimated_bytes(&self) -> u64 {
        use std::mem::size_of;

        let strings_overhead = self.strings.capacity().saturating_mul(size_of::<String>());
        let strings_payload: usize = self.strings.iter().map(|s| s.capacity()).sum();

        let index_overhead = self
            .index
            .capacity()
            .saturating_mul(size_of::<(String, StringId)>());
        strings_overhead
            .saturating_add(strings_payload)
            .saturating_add(index_overhead) as u64
    }
}
```

### With this

```rust
use rustc_hash::{FxHashMap, FxHasher};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StringId(pub u32);

impl std::fmt::Display for StringId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug)]
enum Bucket {
    One(StringId),
    Many(Vec<StringId>),
}

#[derive(Debug, Default)]
pub struct StringPool {
    strings: Vec<String>,
    index: FxHashMap<u64, Bucket>,
}

impl StringPool {
    pub fn new() -> Self {
        let mut pool = Self::default();
        pool.intern("");
        pool
    }

    pub fn intern(&mut self, s: &str) -> StringId {
        let h = hash_str(s);

        if let Some(bucket) = self.index.get_mut(&h) {
            match bucket {
                Bucket::One(existing) => {
                    let id = *existing;
                    if self.strings[id.0 as usize] == s {
                        return id;
                    }
                    let new_id = self.push(s);
                    let mut ids = Vec::with_capacity(2);
                    ids.push(id);
                    ids.push(new_id);
                    *bucket = Bucket::Many(ids);
                    return new_id;
                }
                Bucket::Many(ids) => {
                    for &id in ids.iter() {
                        if self.strings[id.0 as usize] == s {
                            return id;
                        }
                    }
                    let new_id = self.push(s);
                    ids.push(new_id);
                    return new_id;
                }
            }
        }

        let id = self.push(s);
        self.index.insert(h, Bucket::One(id));
        id
    }

    pub fn resolve(&self, id: StringId) -> &str {
        &self.strings[id.0 as usize]
    }

    pub fn strings(&self) -> &[String] {
        &self.strings
    }

    pub fn into_strings(self) -> Vec<String> {
        self.strings
    }

    pub fn len(&self) -> usize {
        self.strings.len()
    }

    pub fn estimated_bytes(&self) -> u64 {
        use std::mem::size_of;

        let strings_overhead = self.strings.capacity().saturating_mul(size_of::<String>());
        let strings_payload: usize = self.strings.iter().map(|s| s.capacity()).sum();
        let index_overhead = self.index.capacity().saturating_mul(size_of::<(u64, Bucket)>());

        let mut collision_payload = 0usize;
        for bucket in self.index.values() {
            if let Bucket::Many(ids) = bucket {
                collision_payload = collision_payload.saturating_add(
                    ids.capacity().saturating_mul(size_of::<StringId>()),
                );
            }
        }

        (strings_overhead + strings_payload + index_overhead + collision_payload) as u64
    }

    fn push(&mut self, s: &str) -> StringId {
        let id = StringId(self.strings.len() as u32);
        self.strings.push(s.to_owned());
        id
    }
}

fn hash_str(s: &str) -> u64 {
    let mut hasher = FxHasher::default();
    s.hash(&mut hasher);
    hasher.finish()
}
```

---

## What this should do to e2e numbers

On “dense unique-string” fixtures (P1/P5), this change:

* removes the **extra allocation + extra copy** per first-seen string
* cuts **string_pool_bytes** materially (which contributes to peak memory accounting) 
* reduces allocator pressure during parse, which typically improves **parse_time_ms**, and therefore **total_time_ms** in your e2e metrics

Even if diff-time stays constant, e2e totals improve because parse is explicitly included in your e2e accounting. 

---

## If you want a second lever after this

If you apply the StringPool fix and still need more e2e improvement, the next biggest “parse-side” win is usually **avoiding the temporary `Vec<ParsedCell>` build-up** in sheet XML parsing (stream directly into a Grid when `<dimension>` is known). That reduces peak memory during parse for very large sheets.

And for “diff-side” wins, your full-scale results show signature-building dominating several tests (parse is 0 there, so it’s isolating diff time). 
That points toward optimizing signature construction and/or enabling the existing parallel path where appropriate.

If you want, I can outline a safe streaming-grid build that keeps the current fallback behavior when `<dimension>` is missing, but the StringPool change above is the cleanest, highest-confidence first step for the specific e2e fixtures you’re running.
