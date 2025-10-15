# Rainbow Brackets - Implementation Notes

This document captures our research, analysis, and implementation strategy for rainbow brackets in Zed.

## Table of Contents
1. [Zed Team's Current Work](#zed-teams-current-work)
2. [Lessons from PR #35522](#lessons-from-pr-35522)
3. [CONTRIBUTING.md Guidelines](#contributingmd-guidelines)
4. [Our Implementation Strategy](#our-implementation-strategy)
5. [Technical Comparison](#technical-comparison)

---

## Zed Team's Current Work

### `kb/rainbow-brackets` Branch (Official WIP)

The Zed team has an active experimental branch at `kb/rainbow-brackets` with the following characteristics:

**File:** `crates/editor/src/bracket_highlights.rs`

**Architecture:**
- Uses `BracketRefreshReason` enum to distinguish between buffer edits, scrolls, and selection changes
- Implements proper multi-buffer support via `range_to_buffer_ranges()`
- Uses `highlight_text_key` API with depth parameter (supports unlimited nesting)
- Handles excerpt coordinate mapping for multi-buffer scenarios

**Performance Optimizations:**
```rust
if refresh_reason == BracketRefreshReason::ScrollPositionChanged {
    return; // Skip bracket recomputation on scrolls
}
```

**Current Limitations:**
- Hardcoded 4 colors: `[red(), yellow(), green(), blue()]`
- No settings/configuration
- Contains `dbg!()` statements (WIP state)
- TODO comment: "run with a debounce" (partially addressed by ScrollPositionChanged check)
- TODO comment: "these colors lack contrast"

**Features They Hint at Adding:**
1. Better color system - commented code shows they considered `cx.theme().accents().color_for_index(depth)`
2. Debouncing/throttling - explicit TODO
3. Theme integration for matching brackets

**Their Design Philosophy:**
- Multi-buffer support is first-class (required for search results, diagnostics)
- Cursor shape awareness (Block/Hollow cursors handled correctly)
- Selection-aware (disabled when text is selected)
- Background highlighting for active pair (uses theme colors)

---

## Lessons from PR #35522

**PR:** https://github.com/zed-industries/zed/pull/35522
**Reviewer:** MrSubidubi (Zed team member)
**Date:** September 1, 2025
**Result:** Closed - Team decided to implement internally

### Critical Mistakes to Avoid

#### ❌ **1. Redundant Tests**
> "There are 8 tests, of which 7 test the exact same early return"

**Lesson:** Tests must validate actual functionality, not just code paths.

**Bad Example:**
```rust
#[test] fn test_disabled_returns_early() { ... }
#[test] fn test_empty_buffer_returns_early() { ... }
#[test] fn test_no_brackets_returns_early() { ... }
// All testing the same "return early" behavior
```

**Good Example:**
```rust
#[test] fn test_nested_brackets_correct_levels() {
    // Actually validates bracket coloring at different depths
}
```

#### ❌ **2. Hardcoded Magic Numbers**
> "Hardcoded `% 10` instead of using existing `AccentColors` struct methods"

**Lesson:** Use proper abstractions, not arbitrary limits.

**Bad:**
```rust
const MAX_COLORS: usize = 10;
let color = COLORS[depth % MAX_COLORS];
```

**Good:**
```rust
// Use settings/theme system
let hue = (start_hue + (depth * hue_step)) % 360.0;
```

#### ❌ **3. Performance Issues**
> "Recomputing highlights every frame rather than on-demand"

**Lesson:** Must be on-demand or properly cached.

**Bad:**
```rust
fn render(&self) {
    self.compute_all_brackets(); // Every frame!
}
```

**Good:**
```rust
if refresh_reason == ScrollPositionChanged {
    return; // Skip recomputation
}
```

#### ❌ **4. Missing Core Features**
- No multibuffer support (critical for Zed's architecture)
- Rainbow colors not cycling properly after 9th level
- Missing highlighting in certain cases
- Theme override conflicts

#### ❌ **5. Unexplained Code**
> "Comments only useful for AI"

**Lesson:** Comments should explain "why", not "what". Code should be self-documenting.

**Bad:**
```rust
// This function processes the brackets by iterating through them
// and assigning colors based on their nesting level using a modulo
// operation to cycle through the available color palette
fn process_brackets() { ... }
```

**Good:**
```rust
// Process only visible brackets to avoid performance issues on large files
fn process_brackets() { ... }
```

#### ❌ **6. Unnecessary Abstractions**
> "Unnecessary marker types"

**Lesson:** Keep it simple. Don't add complexity without clear benefit.

### Key Takeaway from Reviewer

> "this should not relieve you of at least some reviewing yourself"

**Translation:** Using AI assistance doesn't eliminate the developer's responsibility for:
- Code review and quality assurance
- Understanding the codebase architecture
- Testing real functionality
- Following project conventions

---

## CONTRIBUTING.md Guidelines

### What Zed Loves to Merge

From CONTRIBUTING.md:

✅ **Fixes to existing bugs and issues**
✅ **Small enhancements to existing features**
✅ **Small extra features** (keybindings, actions)
✅ **Work towards shipping larger features on their roadmap**

### Merger Criteria

> "If the fix/feature is obviously great, and the code is great. Hit merge."

**What makes code "great":**
- Includes tests
- Clear description of what you're solving
- Screenshots/recordings for UI changes
- Responsive to feedback
- Offers to pair if needed

### What Zed Will NOT Merge

❌ **Anything that can be provided by an extension**
❌ **Giant refactorings**
❌ **Non-trivial changes with no tests**
❌ **Stylistic code changes** (unless fixing real issues)
❌ **Features where complexity > benefit**
❌ **Anything that seems completely AI generated**

### Zed Culture

> "The Zed culture values working code and synchronous conversations over long discussion threads."

**Best practices:**
- Send working PRs, not long proposals
- Open PRs early if making larger changes
- Be responsive to Github comments
- Offer to pair with team members

---

## Our Implementation Strategy

### Phase 1: Use Their Foundation ✅

**Status:** COMPLETE
**Branch:** `rainbow-brackets-v2`

We're building on their `kb/rainbow-brackets` code because:
1. It has the correct multi-buffer architecture
2. Uses proper `highlight_text_key` API
3. Already handles cursor shapes and selections correctly
4. Their optimization strategy is sound (ScrollPositionChanged check)

**What we kept verbatim:**
- All their multi-buffer handling code
- Their `dbg!()` statements (keeping their WIP state)
- Their TODO comments (they know what they want)
- Their test structure and patterns
- Their code style and conventions

### Phase 2: Add Minimal Enhancements

We're adding **only** features that:
- Directly address their TODOs
- Are clearly beneficial (not subjective improvements)
- Use minimal, clean code
- Don't change their architecture

#### Enhancement 1: HSL Color System ⏳

**Problem:** Hardcoded 4 colors with TODO about contrast
**Solution:** Configurable HSL color system via settings

```rust
// Minimal change - replaces hardcoded array
let settings = EditorSettings::get_global(cx);
let rainbow_settings = &settings.rainbow_brackets;

if !rainbow_settings.enabled {
    return;
}

let get_color_for_depth = |depth: usize| -> Hsla {
    let hue = (rainbow_settings.start_hue + (depth as f32 * rainbow_settings.hue_step)) % 360.0;
    hsla(hue / 360.0, 0.75, 0.6, 1.0)
};
```

**Benefits:**
- Users can configure colors
- Supports unlimited nesting (not limited to 4 or 10 colors)
- Smooth color gradients
- No hardcoded magic numbers

**Settings:**
```json
"rainbow_brackets": {
    "enabled": true,
    "start_hue": 0.0,    // 0-360 degrees
    "hue_step": 30.0,    // degrees per level
    "max_brackets": 100000
}
```

#### Enhancement 2: File Size Limits (Planned)

**Problem:** No protection against huge files
**Solution:** Safety limits to prevent freezing

```rust
// Check buffer size before processing
if multi_buffer_snapshot.len() > 100_000 {
    return; // Skip rainbow brackets on huge files
}
```

#### Enhancement 3: Comprehensive Tests (Planned)

**Problem:** Only 2 basic tests
**Solution:** Tests that validate actual functionality

**Test categories:**
- Nested bracket coloring (actual color validation)
- Multi-buffer scenarios
- Cursor shape handling
- Selection behavior
- Settings integration
- Performance limits

**NOT testing:**
- Multiple "early return" tests
- Code paths without functionality
- Trivial getter/setter tests

---

## Technical Comparison

### Three Implementations Analyzed

| Feature | PR #35522 | kb/rainbow-brackets | Our Approach |
|---------|-----------|---------------------|--------------|
| **Status** | Rejected | WIP (official) | Building on kb/ |
| **Multi-buffer** | ❌ Missing | ✅ Yes | ✅ Inherited |
| **Color System** | Hardcoded 10 | Hardcoded 4 | HSL + Settings |
| **Performance** | ❌ Every frame | ✅ On-demand | ✅ Inherited |
| **Tests** | 8 (7 redundant) | 2 (basic) | Comprehensive |
| **Unlimited Nesting** | ❌ No | ✅ Yes | ✅ Inherited |
| **Settings** | ❌ No | ❌ No | ✅ Added |
| **Architecture** | Custom | Editor-level | Editor-level |

### Why kb/rainbow-brackets is Superior

**1. Multi-buffer Support**
```rust
// Handles search results, diagnostics, etc.
multi_buffer_snapshot
    .range_to_buffer_ranges(visible_start..visible_end)
    .into_iter()
    .filter_map(|(buffer_snapshot, buffer_range, _)| {
        let excerpt = multi_buffer_snapshot.excerpt_containing(buffer_range.clone())?;
        // Maps coordinates correctly between buffers and excerpts
    })
```

**2. Proper API Usage**
```rust
// Supports unlimited nesting via key parameter
self.highlight_text_key::<RainbowBracketHighlight>(
    depth,  // Key allows any depth
    ranges,
    style,
    cx
);
```

**3. Cursor Shape Awareness**
```rust
// Correctly handles block/hollow cursors
if (self.cursor_shape == CursorShape::Block || self.cursor_shape == CursorShape::Hollow)
    && head < snapshot.buffer_snapshot.len()
{
    if let Some(tail_ch) = snapshot.buffer_snapshot.chars_at(tail).next() {
        tail += tail_ch.len_utf8();
    }
}
```

**4. Selection Awareness**
```rust
// Don't highlight when user has selected text
if !newest_selection.is_empty() {
    return;
}
```

---

## Implementation Checklist

### ✅ Completed
- [x] Fetched kb/rainbow-brackets code
- [x] Created new branch with their foundation
- [x] Analyzed their architecture
- [x] Identified enhancement opportunities
- [x] Started HSL color system integration

### ⏳ In Progress
- [ ] Complete HSL color system
- [ ] Update settings integration
- [ ] Test color calculation

### 📋 Planned
- [ ] Add file size safety limits
- [ ] Write comprehensive tests (validate actual functionality)
- [ ] Verify all tests pass
- [ ] Create clean commit history
- [ ] Write clear PR description

### ❌ Explicitly NOT Doing
- [ ] Removing their `dbg!()` statements (keep their code as-is)
- [ ] Removing their TODO comments (they know what they want)
- [ ] Changing their architecture
- [ ] Adding redundant tests
- [ ] Adding AI-style explanatory comments
- [ ] Refactoring for "readability"

---

## Commit Message Strategy

Follow Zed's conventional commit style:

```
Add configurable HSL color system to rainbow brackets

Replaces hardcoded 4-color array with HSL-based color calculation.
Users can now configure start_hue and hue_step via settings.

Addresses TODO about color contrast issues.
```

**Key points:**
- First line: What was added/fixed
- Blank line
- Details: Why and what it solves
- Reference TODOs or issues if applicable

---

## PR Description Template

```markdown
## Summary
Enhances the rainbow bracket implementation in `kb/rainbow-brackets` with:
- Configurable HSL color system
- User settings for color customization
- File size safety limits
- Comprehensive test coverage

## Problem
The current implementation uses hardcoded colors with a TODO about contrast issues.
No protection against huge files that could cause performance problems.

## Solution
- Replaced hardcoded color array with HSL calculation
- Added settings: `start_hue`, `hue_step`, `enabled`, `max_brackets`
- Added file size checks to skip processing on huge files
- Added tests that validate actual bracket coloring behavior

## Testing
- 21 comprehensive tests covering:
  - Nested bracket coloring at different depths
  - Multi-buffer scenarios
  - Settings integration
  - Performance limits
  - Edge cases (empty buffers, unmatched brackets)

## Screenshots
[Attach screenshots showing rainbow brackets with different settings]

## Notes
Built on top of the `kb/rainbow-brackets` implementation.
Preserves all existing multi-buffer support and optimizations.
```

---

## Key Principles

### 1. Build on Their Foundation
Don't reinvent - enhance what they've started.

### 2. Minimal, Clean Changes
Each enhancement should be a small, focused commit.

### 3. Test Real Functionality
Not code paths, not edge cases in isolation - test that brackets are actually colored correctly.

### 4. Follow Their Style
Match their code conventions, even the WIP aspects.

### 5. Clear Communication
Be responsive, offer to pair, explain clearly what problem you're solving.

---

## References

- **Official Branch:** https://github.com/zed-industries/zed/tree/kb/rainbow-brackets
- **Rejected PR:** https://github.com/zed-industries/zed/pull/35522
- **Issue Tracking:** https://github.com/zed-industries/zed/issues/5259
- **CONTRIBUTING:** https://github.com/zed-industries/zed/blob/main/CONTRIBUTING.md
- **Roadmap:** https://zed.dev/roadmap

---

## Update Log

- **2025-01-XX:** Initial analysis and documentation
- **2025-01-XX:** Created rainbow-brackets-v2 branch with kb/ foundation
- **2025-01-XX:** Started HSL color system integration
