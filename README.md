## `gfontapi`

Something I build because I wanted to learn some Rust, and also because I got tired of manually downloading and running font converters for every google font I wanted to use in my web applications.

### Usage

```
Usage: gfontapi [OPTIONS] "[fontname]"

Options
  -t, --target-dir <TARGET_DIR>  target directory, defaults to ./fonts.
  -a, --api-key <API_KEY>        google api key generated from developer console, can also be set as `EXPORT GFONT_API_KEY=<API_KEY>`
  -h, --help                     Print help
  -V, --version                  Print version
```

Something to note, you have to surround the font with double (or single) quotes especially for multi-word fonts. *Working on a fix for it.*

**Demo**

![gfontapi.gif](./screencasts/gfontapi.mp4 "Usage")

Supports adding any google font, creates a `fonts.css` file to add support for each font variant and style.

### TODOS

- [ ] Add support for variable fonts (although I'm not sure this is actually better in terms of font optimisation).
- [ ] Add more commands to the CLI (remove, add, inspect).
- [ ] Make the code more idiomatic and easier to manage.
- [ ] Stop shipping the `woff2_compress` binary (to convert the `ttf` font to `woff2` format) with the application, support building when required.