# Git integration

## Configure difftool (cell-level diff output)

Add this to your ~/.gitconfig:

```
[difftool "excel-diff"]
    cmd = excel-diff diff --git-diff "$LOCAL" "$REMOTE"
```

Then run:

```
git difftool --tool=excel-diff
```

## Configure diff driver (structure-only textconv)

Add this to your ~/.gitconfig:

```
[diff "xlsx"]
    textconv = excel-diff info
    binary = true
```

Add this to your repo's .gitattributes:

```
*.xlsx diff=xlsx
*.xlsm diff=xlsx
```

Then run:

```
git diff --textconv
```

