# klok

simple terminal clock.

<img src="https://raw.githubusercontent.com/amatagonsk/klok/master/img/demo.avif" width="50%" />

## installation
### crate.io
```
cargo install klok
```

### github
```
cargo install --git https://github.com/amatagonsk/klok.git
```

or download from [release](https://github.com/amatagonsk/klok/releases)

## --help

```
Usage: klok [OPTIONS]

Options:
  -s, --size <SIZE>  [possible values: full, half, quadrant, sextant, analog]
  -h, --help         Print help
  -V, --version      Print version
```

and `tab` or `middle mouse button` key change size.

## mojibake? (at sextant size)

i guess your terminal font is not support [Symbols for Legacy Computing - Wikipedia](https://en.wikipedia.org/wiki/Symbols_for_Legacy_Computing).

as far as I know, [microsoftcascadia-code](https://github.com/microsoft/cascadia-code) font is supported.(but nerd font is not good)

## nice framework

[ratatui](https://github.com/ratatui-org/ratatui)  
[tui-big-text](https://github.com/joshka/tui-big-text)  
