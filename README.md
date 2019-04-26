# Summary

This is the basic start to implementing a full Bell 103 modem. Currently, only the demodulating
portion is implemented and is not fully featured.

It is currently capable of decoding 48000 kilosample/s 16-bit single-channel WAV files in
little-endian Microsoft PCM format. The file contents must be encoded using the answering
frequencies of the 9N1 Bell 103 protocol at 300 bits per second. The bytes must also be
packed tight with no lead-in or filtering.

There are options for changing the sampling rate, and filter length as well as using the
origin frequencies instead of answering but these have not been tested.

Future additions could include:
- Adding syncronization that finds the start of each valid byte
- Adding realtime functionality to allow full duplex
- Adding the ability to handle alternative formats, band limitations, noise, etc.

# Usage

```
USAGE:
    bell103_demodulator [FLAGS] [OPTIONS] <file> [output]

FLAGS:
    -h, --help       Prints help information
    -o, --origin     Use originating mark/space frequencies (default uses answering frequencies
    -V, --version    Prints version information

OPTIONS:
    -l, --filter_length <filter_length>    Goertzel filter length N [default: 160]
    -s, --sampling_rate <sampling_rate>    Audio sampling rate [default: 48000]

ARGS:
    <file>      The PCM WAV file to be decoded
    <output>    The output file to store the message
```

# Examples

There are files in the examples folder used when developing this that decode messages.

e.g.

```
$ bell103_demodulator examples/fortune.wav
Your nature demands love and your happiness depends on it.
```
