---
kind: routine
description: Capture a URL to a PNG or PDF file. Use when the user asks to screenshot, capture, or render
  a page to an image.
uses:
- usage: '[[run-usage]]'
  when:
  - screenshot a page
  - capture a web page
  - render url to image
  - snapshot a site
  - page to pdf
  tags:
  - screenshot
  - browser
  - image
  - artifact
  - binary
---

## Inputs

No prerequisites beyond the commands named below. Recipe parameters are named in Do; supply paths and arguments for the current task.

## Do

Binary output: the recipe writes the file and prints `ARTIFACT <path>`; read the
file at that path. Delegates to `playwright`; swap for `chromium --headless`,
`puppeteer`, etc.

```just
# capture a URL to a PNG
shot url out="dist/shot.png":
  @mkdir -p dist
  npx playwright screenshot --wait-for-timeout 1000 {{url}} {{out}}
  @echo "ARTIFACT {{out}}"

# capture a URL to a PDF (callable by name)
pdf url out="dist/shot.pdf":
  @mkdir -p dist
  npx playwright pdf {{url}} {{out}}
  @echo "ARTIFACT {{out}}"
```

## Check

Every recipe exits 0 and produces the output or files its row in Do describes. `shot` is the default recipe.

## Failure

Report a missing prerequisite or failed command. Resolve the failure before continuing dependent steps.
