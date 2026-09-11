# Unreal Piano

**WIP physical-modeling-based piano synthesizer**
Right now it's a total crap. It sounds like a stupid Ukulele or some sort of Koto
Which is sort of a good sign I guess: piano is a string instrument, and so are these.
But when you make Lasagna you usually want to get a Lasagna, not a Ukulele or Koto

## Vision

Making a great sounding piano is a number 1 priority with this synth, but since this is not a real piano anyway, why not say screw it and push beyond simply synthesizing the sound of a piano, go nuts with sound design and textures, something that is impossible with a real piano

## Credits

The physical modeling approach is based on research by **Haifan Xie**:

* [Physical modeling research paper](https://arxiv.org/html/2409.03481v3#S1)

This project aims to replicate and build upon the techniques presented in the paper.

## Building

Requires [cargo-nice-plug](https://codeberg.org/RustAudio/nice-plug).

```bash
cargo nice-plug bundle shade --release
```
