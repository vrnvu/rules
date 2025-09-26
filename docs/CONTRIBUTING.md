# Contributing

## Branch & PR
- Use forks or `user/feature` branches
- Assign a single reviewer; reviewer owns moving PR to done

## Checks (Not Rocket Science Rule)
- Run `make check` locally; CI runs the same
  - fmt (rustfmt)
  - lint (clippy -D warnings)
  - tests (fast by default)
  - large blob guard

## Rules Edits
- Keep rules focused; add When/Then frontmatter
- Use subfolders: `project/`, `assertions/`, `style/`, `testing/`
- Visibility: see `project/rules-visibility.mdc`

