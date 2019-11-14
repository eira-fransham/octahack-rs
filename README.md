# Octahack

An embeddable, precise and efficient modular music (or anything else you want) system. As the WIP name implies, it's heavily inspired by Elektron's Octatrack, a ridiculously efficient hardware sampler/music workstation. This library is designed quite differently, however. Essentially it's a digital modular rack, but designed to be usable in a performance setting. The north star for this is to be efficient enough for a performer to be able to start from a completely blank slate and play an entire electronic music set. It should be controllable 100% with a MIDI controller like the APC or Launchpad, or even better to compile as a custom OS for an Arduino or Raspberry Pi-based custom piece of hardware.

This README is basically just a way for me to get all my thoughts out and organised, so it might get out-of-date as the project evolves, but it will give a pretty decent overview of what I'm aiming for here.

## Concepts

### Rack

This is the "world" of the Octahack. Each project is a single Rack, which has a maximum size determined when the project is compiled. You only have one "track", as with many rack-based DAWs you implement multiple tracks by passing different subsets of components into different elements of a mixer. You start with just the track inputs and outputs, which are defined by whatever platform you're running on. For example, if you compiled the Octahack software to run on a real Octatrack (which is unlikely but theoretically possible), the inputs would be the individual A/B/C/D audio inputs and the MIDI input. The outputs would be the MIDI out, Cue L, Cue R, Main L, Main R, and perhaps the headphone out would be separate too depending on how the Octatrack is wired internally. The precise input mapping is still up for debate, but the current image in my head is a "component" button which is the "super" key for component-related activities. Pressing and releasing "component" on its own allows you to replace the current component, "component + left" adds a component before the current one, and "component + right" adds a component after the current one. Probably this "new/edit component" menu would be where you would find the option to add a new group too, where adding a new group would just create a group containing only the current component, no matter which of "edit/add before/add after" was selected.

### Component

A component is a mapping from inputs to outputs, and acts in a "push/pull" manner. Every tick, 44100 times a second (or whatever the output frequency is set to), each component is updated and can change internal state, then each output of the rack is calculated by querying the components that it is wired to, these components can then query the value of any of their inputs which query the outputs that _they're_ wired to and so forth. The reason to have this dual system is that some components need to constantly update, whereas others can avoid calculating a lot of the time. A delay/reverb component wants to consume input even when it's not outputting anything because it's stateful, and only updating it when its outputs are being output to the outside world will lead to weird and surprising behaviour. With an audio recorder component the situation would be even worse. Conversely, some other components could be made far simpler and more efficient by only calculating inputs that need to be calculated.

### I/O

The data that flows through components is typed, where inputs can only be wired to outputs of corresponding types. Inputs are either "continuous", roughly akin to the analogue I/O in Eurorack, or MIDI. MIDI is separate instead of being split into analogue components since any components that need to work with MIDI can far easier process the commands instead of having separate I/O for every CC, every note (which would also mean we'd have to have a set maximum number of voices), et cetera. You could also imagine that certain components could understand nonstandard MIDI messages while others let them pass through. It's possible that in the future I'll allow continuous inputs to be directly wired to elements of a MIDI output, transparently inserting a stateful MIDI-to-analogue converter on an as-needed basis. This way, if only CC 5 (for example) is used, only the work to maintain state for CC 5 will be done, rather than maintaining state for every possible part of the MIDI space.

Outputs can produce an arbitrary number of values per tick. For audio outputs this is understood to be audio channels, which means that we can write components in a number-of-channels-agnostic way and it means that left and right components don't need to be wired independently. This is important in a performance setting.

### Parameters

Unlike in a traditional modular rack, where any parameters that should be controlled need a corresonding CV voltage input, instead parameters are first-class and can be wired to an output of the correct type. Parameters being first-class means we can implement things like the following:

- Octatrack-style scenes with crossfading
- Locking parameters together (so changing one will always change another)
- Adding parameters to groups that can control one-or-more parameters of subcomponents

Plus it means that automating parameters is more lightweight, where any parameter _can_ be automated, but you don't have to clutter up the list of inputs with control inputs. When a parameter is wired to an output, the output being at the minimum possible value will give it its "natural value" (i.e. the value it would have if it wasn't wired at all), whereas you can also set the "wired value", which will be the value that the parameter will have if the output is at the maximum possible value.

### File editors

In order to allow components working with MIDI to interact better, instead of having a "sequencer" which emits MIDI, it would be better to only have a MIDI _player_ component type but include a MIDI file editor, with access to this editor for any MIDI file currently in use to never be more than a couple button-presses away. Sequencing arbitrary parameters like the Octatrack's "parameter locks" is done by wiring the MIDI player into a splitter which gives you gate/note/CC as separate outputs and then wiring those CC params to the desired parameters. This means that saving and loading sequences is no different from saving and loading MIDI files, and you can easily swap a live-programmed sequence out for a pre-made MIDI file, save a live-programmed sequence for later import into a desktop DAW, etc. Building our whole sequencing system around MIDI also forces us to treat MIDI sequenced within the system no differently to MIDI input from an external device, which makes us play nicer with hardware sequencers.

### Recording

Since this is a system built for performance, recording has to be first-class. I think that, similar to how in Octatrack flex machines can play recordings just like any other flex slot, recordings should just be files and you should be able to work with them the same as any file. I don't know yet whether you should be able to have arbitrarily-expandable (i.e. infinite until you run out of memory) recording buffers or whether you should have to specify maximum recording time upfront, like the Octatrack. Certainly I think that you should be able to create an arbitrarily-high number of recording buffers as long as you don't run out of memory, instead of having both the size and number of the buffers fixed. The recording itself is just done with a component, rather than having any special place in the UI, since we'll have to have some way to choose an arbitrary output to record with anyway.

While audio recording is obviously the most immediately-clear use of this, MIDI recording should use the exact same system.

### Grouping

Groups are a special kind of component, which are a way to collect components together and abstract away details. You create a group with a single component, but can expand its size to hold any number of components. Groups have any number of inputs and any number of outputs (TODO: perhaps with a maximum dictated by the number that can be easily represented in the UI?), and unlike component inputs/outputs these are "polymorphic", which means they can be any type. Any wire from inside to outside the group or vice versa must pass through the inputs and outputs of the group itself, but to make things simpler it'll probably be possible to directly wire an output of any component to any other component, with new inputs/outputs on the group and the path between the two components being generated automatically. It should be possible to save a group to a file and load it back into any project, allowing users to save and load synths, effects or even whole tracks, which should allow a DJ-like workflow where you can have two groups at once, both wired into a mixer, fade into the second one from the first, then delete the first and load in a new group (which is the next track in the set).

Because of this system of saving/loading groups, I think that a slot-based system for file access like the Octatrack's is undesirable. Although it's useful for quickly swapping out files in many places at once during a performance, I think that you could get the same benefit by combining a few smaller features:

- First, any time a component needs a file, it references it by path instead of by slot
- If you want two components to share the same file, you lock their "file" parameters together as mentioned in the earlier parameters section
- We allow a "quick view" which shows all the files currently in use in the project, with files that are locked together shown as a single entry, but files that happen to be the same but are _not_ locked together shown as separate entries
- When editing a file parameter, you are presented with the quick view along with an option to choose from the file system, and choosing from the quick view just locks the parameters together

### Other helpers

Some ideas for functionality that I think would be useful:

- Quantise edits to beat: either quantise the next edit to a specified amount of time in beats or quantise _all_ edits until told otherwise. There could be a shortcut to quantise the next edit to whatever the last edit quantise was. This is useful for things like f.e. switching a wire from output A to output B on the end of the next bar, or to do quantised recording when combined with...
- Immediately trigger input: for recorders and such, we want as much as possible of their interface to be controllable by other components, since it would be useful for automation purposes. If we simply have their interface be gate inputs but allow a simple and fast UI for triggering inputs, this could allow that easily. It could also allow users to connect these inputs to MIDI triggers if they like, which would make things like looping when you press a foot pedal possible.
- Shortcut parameters: up to 8 parameters that are always quickly available, which can be set by a user.
