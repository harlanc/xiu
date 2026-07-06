/******************************************************************************
 *
 * Copyright (C) 2020 The Android Open Source Project
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at:
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 *****************************************************************************
 * Originally developed and contributed by Ittiam Systems Pvt. Ltd, Bangalore
 */

#include <stdint.h>
#include "aacdecoder_lib.h"

constexpr uint8_t kNumberOfLayers = 1;
constexpr uint8_t kMaxChannelCount = 8;

class Codec {
 public:
  Codec() = default;
  ~Codec() { deInitDecoder(); }
  bool initDecoder();
  void decodeFrames(UCHAR *data, UINT size);
  void deInitDecoder();

 private:
  HANDLE_AACDECODER mAacDecoderHandle = nullptr;
  AAC_DECODER_ERROR mErrorCode = AAC_DEC_OK;
};

bool Codec::initDecoder() {
  mAacDecoderHandle = aacDecoder_Open(TT_MP4_ADIF, kNumberOfLayers);
  if (!mAacDecoderHandle) {
    return false;
  }
  return true;
}

void Codec::deInitDecoder() {
  aacDecoder_Close(mAacDecoderHandle);
  mAacDecoderHandle = nullptr;
}

void Codec::decodeFrames(UCHAR *data, UINT size) {
  while (size > 0) {
    UINT inputSize = size;
    UINT valid = size;
    mErrorCode = aacDecoder_Fill(mAacDecoderHandle, &data, &inputSize, &valid);
    if (mErrorCode != AAC_DEC_OK) {
      ++data;
      --size;
    } else {
      INT_PCM outputBuf[2048 * kMaxChannelCount];
      aacDecoder_DecodeFrame(mAacDecoderHandle, outputBuf, 2048 * kMaxChannelCount, 0);
      if (valid >= inputSize) {
        return;
      }
      UINT offset = inputSize - valid;
      data += offset;
      size = valid;
    }
  }
}

extern "C" int LLVMFuzzerTestOneInput(const uint8_t *data, size_t size) {
  Codec *codec = new Codec();
  if (!codec) {
    return 0;
  }
  if (codec->initDecoder()) {
    codec->decodeFrames((UCHAR *)(data), static_cast<UINT>(size));
  }
  delete codec;
  return 0;
}
