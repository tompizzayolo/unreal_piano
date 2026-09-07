# Unreal Piano

**WIP physical-modeling-based piano synthesizer**
Currently it is a total crap

## Vision

Push beyond simply synthesizing the sound of a piano and go nuts with sound design and textures 

## Credits

The physical modeling approach is based on research by **Haifan Xie**:

* [Physical modeling research paper](https://arxiv.org/html/2409.03481v3#S1)

This project aims to replicate and build upon the techniques presented in the paper.

## Building

Requires [cargo-nice-plug](https://github.com/robbert-vdh/nih-plug).

```bash
cargo nice-plug bundle unreal_piano --release
```

> **Status:** Work in progress. Expect things to break.
