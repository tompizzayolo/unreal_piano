# Unreal Piano

**WIP physical-modeling-based piano synthesizer**
Currently it's a total crap

## Vision

Making a great sounding piano is a number 1 priority with this synth, but since this is not a real piano anyway, why not say screw it and push beyond simply synthesizing the sound of a piano, go nuts with sound design and textures, something that is impossible with a real piano

## Credits

The physical modeling approach is based on research by **Haifan Xie**:

* [Physical modeling research paper](https://arxiv.org/html/2409.03481v3#S1)

This project aims to replicate and build upon the techniques presented in the paper.

## Building

Requires [cargo-nice-plug](https://codeberg.org/RustAudio/nice-plug).

```bash
cargo nice-plug bundle unreal_piano --release
```

> **Status:** Work in progress. Expect things to break.
