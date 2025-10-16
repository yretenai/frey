<!--
SPDX-FileCopyrightText: 2025 Ada Freya Ahmed (neptuwunium)
SPDX-License-Identifier: EUPL-1.2
-->

# Frey

Luminous Engine Tools and QoL, focusing on Forspoken.

## Project Scope

The project exists in three main parts.

## witch

Witch is the core library of the project, it handles most file types including conversion.

### witch-common

Witch-common is mostly for reading structures from memory and the overlap between that and witch itself.

Primarily used by sorcery-introspection and sorcery.

### witch-macro

As of current, unused. will be used for proc_macros should we need them (likely for precalculating hashes.)

## scarlet

Scarlet is a third-party launcher for the game itself, allowing the project to interact with and modify game behavior.

### scarlet-module

Injected into the game and interacts with various game frameworks.

## sorcery

Sorcery is a series of frontend graphical and command line interfaces that sit on top of Witch.

### sorcery-introspection

Reads and dumps structures object and module registries from the game.

### sorcery-viewer

Model viewer and Texture Viewer.
