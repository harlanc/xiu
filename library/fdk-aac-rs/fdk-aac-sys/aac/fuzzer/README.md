# Fuzzer for libFraunhoferAAC decoder

## Plugin Design Considerations
The fuzzer plugin for aac decoder is designed based on the understanding of the
codec and tries to achieve the following:

##### Maximize code coverage

This fuzzer makes use of the following config parameters:
1. Transport type (parameter name: `TRANSPORT_TYPE`)

| Parameter| Valid Values| Configured Value|
|------------- |-------------| ----- |
| `TRANSPORT_TYPE` | 0.`TT_UNKNOWN  ` 1.`TT_MP4_RAW ` 2.`TT_MP4_ADIF ` 3.`TT_MP4_ADTS ` 4.`TT_MP4_LATM_MCP1 ` 5.`TT_MP4_LATM_MCP0  ` 6.`TT_MP4_LOAS ` 7.`TT_DRM ` | `TT_MP4_ADIF ` |

Note: Value of `TRANSPORT_TYPE` could be set to any of these values.
It is set to `TT_MP4_ADIF` in the fuzzer plugin.

##### Maximize utilization of input data
The plugin feeds the entire input data to the codec using a loop.
 * If the decode operation was successful, the input is advanced by an
   offset calculated using valid bytes.
 * If the decode operation was un-successful, the input is advanced by 1 byte
   till it reaches a valid frame or end of stream.

This ensures that the plugin tolerates any kind of input (empty, huge,
malformed, etc) and doesnt `exit()` on any input and thereby increasing the
chance of identifying vulnerabilities.

## Build

This describes steps to build aac_dec_fuzzer binary.

## Android

### Steps to build
Build the fuzzer
```
  $ mm -j$(nproc) aac_dec_fuzzer
```

### Steps to run
Create a directory CORPUS_DIR and copy some aac files to that folder.
Push this directory to device.

To run on device
```
  $ adb sync data
  $ adb shell /data/fuzz/arm64/aac_dec_fuzzer/aac_dec_fuzzer CORPUS_DIR
```
To run on host
```
  $ $ANDROID_HOST_OUT/fuzz/x86_64/aac_dec_fuzzer/aac_dec_fuzzer CORPUS_DIR
```

## References:
 * http://llvm.org/docs/LibFuzzer.html
 * https://github.com/google/oss-fuzz

