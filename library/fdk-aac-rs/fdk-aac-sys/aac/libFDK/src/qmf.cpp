/* -----------------------------------------------------------------------------
Software License for The Fraunhofer FDK AAC Codec Library for Android

© Copyright  1995 - 2018 Fraunhofer-Gesellschaft zur Förderung der angewandten
Forschung e.V. All rights reserved.

 1.    INTRODUCTION
The Fraunhofer FDK AAC Codec Library for Android ("FDK AAC Codec") is software
that implements the MPEG Advanced Audio Coding ("AAC") encoding and decoding
scheme for digital audio. This FDK AAC Codec software is intended to be used on
a wide variety of Android devices.

AAC's HE-AAC and HE-AAC v2 versions are regarded as today's most efficient
general perceptual audio codecs. AAC-ELD is considered the best-performing
full-bandwidth communications codec by independent studies and is widely
deployed. AAC has been standardized by ISO and IEC as part of the MPEG
specifications.

Patent licenses for necessary patent claims for the FDK AAC Codec (including
those of Fraunhofer) may be obtained through Via Licensing
(www.vialicensing.com) or through the respective patent owners individually for
the purpose of encoding or decoding bit streams in products that are compliant
with the ISO/IEC MPEG audio standards. Please note that most manufacturers of
Android devices already license these patent claims through Via Licensing or
directly from the patent owners, and therefore FDK AAC Codec software may
already be covered under those patent licenses when it is used for those
licensed purposes only.

Commercially-licensed AAC software libraries, including floating-point versions
with enhanced sound quality, are also available from Fraunhofer. Users are
encouraged to check the Fraunhofer website for additional applications
information and documentation.

2.    COPYRIGHT LICENSE

Redistribution and use in source and binary forms, with or without modification,
are permitted without payment of copyright license fees provided that you
satisfy the following conditions:

You must retain the complete text of this software license in redistributions of
the FDK AAC Codec or your modifications thereto in source code form.

You must retain the complete text of this software license in the documentation
and/or other materials provided with redistributions of the FDK AAC Codec or
your modifications thereto in binary form. You must make available free of
charge copies of the complete source code of the FDK AAC Codec and your
modifications thereto to recipients of copies in binary form.

The name of Fraunhofer may not be used to endorse or promote products derived
from this library without prior written permission.

You may not charge copyright license fees for anyone to use, copy or distribute
the FDK AAC Codec software or your modifications thereto.

Your modified versions of the FDK AAC Codec must carry prominent notices stating
that you changed the software and the date of any change. For modified versions
of the FDK AAC Codec, the term "Fraunhofer FDK AAC Codec Library for Android"
must be replaced by the term "Third-Party Modified Version of the Fraunhofer FDK
AAC Codec Library for Android."

3.    NO PATENT LICENSE

NO EXPRESS OR IMPLIED LICENSES TO ANY PATENT CLAIMS, including without
limitation the patents of Fraunhofer, ARE GRANTED BY THIS SOFTWARE LICENSE.
Fraunhofer provides no warranty of patent non-infringement with respect to this
software.

You may use this FDK AAC Codec software or modifications thereto only for
purposes that are authorized by appropriate patent licenses.

4.    DISCLAIMER

This FDK AAC Codec software is provided by Fraunhofer on behalf of the copyright
holders and contributors "AS IS" and WITHOUT ANY EXPRESS OR IMPLIED WARRANTIES,
including but not limited to the implied warranties of merchantability and
fitness for a particular purpose. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR
CONTRIBUTORS BE LIABLE for any direct, indirect, incidental, special, exemplary,
or consequential damages, including but not limited to procurement of substitute
goods or services; loss of use, data, or profits, or business interruption,
however caused and on any theory of liability, whether in contract, strict
liability, or tort (including negligence), arising in any way out of the use of
this software, even if advised of the possibility of such damage.

5.    CONTACT INFORMATION

Fraunhofer Institute for Integrated Circuits IIS
Attention: Audio and Multimedia Departments - FDK AAC LL
Am Wolfsmantel 33
91058 Erlangen, Germany

www.iis.fraunhofer.de/amm
amm-info@iis.fraunhofer.de
----------------------------------------------------------------------------- */

/******************* Library for basic calculation routines ********************

   Author(s):   Markus Lohwasser, Josef Hoepfl, Manuel Jander

   Description: QMF filterbank

*******************************************************************************/

/*!
  \file
  \brief  Complex qmf analysis/synthesis
  This module contains the qmf filterbank for analysis [
  cplxAnalysisQmfFiltering() ] and synthesis [ cplxSynthesisQmfFiltering() ]. It
  is a polyphase implementation of a complex exponential modulated filter bank.
  The analysis part usually runs at half the sample rate than the synthesis
  part. (So called "dual-rate" mode.)

  The coefficients of the prototype filter are specified in #qmf_pfilt640 (in
  sbr_rom.cpp). Thus only a 64 channel version (32 on the analysis side) with a
  640 tap prototype filter are used.

  \anchor PolyphaseFiltering <h2>About polyphase filtering</h2>
  The polyphase implementation of a filterbank requires filtering at the input
  and output. This is implemented as part of cplxAnalysisQmfFiltering() and
  cplxSynthesisQmfFiltering(). The implementation requires the filter
  coefficients in a specific structure as described in #sbr_qmf_64_640_qmf (in
  sbr_rom.cpp).

  This module comprises the computationally most expensive functions of the SBR
  decoder. The accuracy of computations is also important and has a direct
  impact on the overall sound quality. Therefore a special test program is
  available which can be used to only test the filterbank: main_audio.cpp

  This modules also uses scaling of data to provide better SNR on fixed-point
  processors. See #QMF_SCALE_FACTOR (in sbr_scale.h) for details. An interesting
  note: The function getScalefactor() can constitute a significant amount of
  computational complexity - very much depending on the bitrate. Since it is a
  rather small function, effective assembler optimization might be possible.

*/

#include "qmf.h"

#include "FDK_trigFcts.h"
#include "fixpoint_math.h"
#include "dct.h"

#define QSSCALE (0)
#define FX_DBL2FX_QSS(x) (x)
#define FX_QSS2FX_DBL(x) (x)

/* moved to qmf_pcm.h: -> qmfSynPrototypeFirSlot */
/* moved to qmf_pcm.h: -> qmfSynPrototypeFirSlot_NonSymmetric */
/* moved to qmf_pcm.h: -> qmfSynthesisFilteringSlot */

#ifndef FUNCTION_qmfAnaPrototypeFirSlot
/*!
  \brief Perform Analysis Prototype Filtering on a single slot of input data.
*/
static void qmfAnaPrototypeFirSlot(
    FIXP_DBL *analysisBuffer,
    INT no_channels, /*!< Number channels of analysis filter */
    const FIXP_PFT *p_filter, INT p_stride, /*!< Stride of analysis filter    */
    FIXP_QAS *RESTRICT pFilterStates) {
  INT k;

  FIXP_DBL accu;
  const FIXP_PFT *RESTRICT p_flt = p_filter;
  FIXP_DBL *RESTRICT pData_0 = analysisBuffer + 2 * no_channels - 1;
  FIXP_DBL *RESTRICT pData_1 = analysisBuffer;

  FIXP_QAS *RESTRICT sta_0 = (FIXP_QAS *)pFilterStates;
  FIXP_QAS *RESTRICT sta_1 =
      (FIXP_QAS *)pFilterStates + (2 * QMF_NO_POLY * no_channels) - 1;
  INT pfltStep = QMF_NO_POLY * (p_stride);
  INT staStep1 = no_channels << 1;
  INT staStep2 = (no_channels << 3) - 1; /* Rewind one less */

  /* FIR filters 127..64 0..63 */
  for (k = 0; k < no_channels; k++) {
    accu = fMultDiv2(p_flt[0], *sta_1);
    sta_1 -= staStep1;
    accu += fMultDiv2(p_flt[1], *sta_1);
    sta_1 -= staStep1;
    accu += fMultDiv2(p_flt[2], *sta_1);
    sta_1 -= staStep1;
    accu += fMultDiv2(p_flt[3], *sta_1);
    sta_1 -= staStep1;
    accu += fMultDiv2(p_flt[4], *sta_1);
    *pData_1++ = (accu << 1);
    sta_1 += staStep2;

    p_flt += pfltStep;
    accu = fMultDiv2(p_flt[0], *sta_0);
    sta_0 += staStep1;
    accu += fMultDiv2(p_flt[1], *sta_0);
    sta_0 += staStep1;
    accu += fMultDiv2(p_flt[2], *sta_0);
    sta_0 += staStep1;
    accu += fMultDiv2(p_flt[3], *sta_0);
    sta_0 += staStep1;
    accu += fMultDiv2(p_flt[4], *sta_0);
    *pData_0-- = (accu << 1);
    sta_0 -= staStep2;
  }
}
#endif /* !defined(FUNCTION_qmfAnaPrototypeFirSlot) */

#ifndef FUNCTION_qmfAnaPrototypeFirSlot_NonSymmetric
/*!
  \brief Perform Analysis Prototype Filtering on a single slot of input data.
*/
static void qmfAnaPrototypeFirSlot_NonSymmetric(
    FIXP_DBL *analysisBuffer,
    int no_channels, /*!< Number channels of analysis filter */
    const FIXP_PFT *p_filter, int p_stride, /*!< Stride of analysis filter    */
    FIXP_QAS *RESTRICT pFilterStates) {
  const FIXP_PFT *RESTRICT p_flt = p_filter;
  int p, k;

  for (k = 0; k < 2 * no_channels; k++) {
    FIXP_DBL accu = (FIXP_DBL)0;

    p_flt += QMF_NO_POLY * (p_stride - 1);

    /*
      Perform FIR-Filter
    */
    for (p = 0; p < QMF_NO_POLY; p++) {
      accu += fMultDiv2(*p_flt++, pFilterStates[2 * no_channels * p]);
    }
    analysisBuffer[2 * no_channels - 1 - k] = (accu << 1);
    pFilterStates++;
  }
}
#endif /* FUNCTION_qmfAnaPrototypeFirSlot_NonSymmetric */

/*!
 *
 * \brief Perform real-valued forward modulation of the time domain
 *        data of timeIn and stores the real part of the subband
 *        samples in rSubband
 *
 */
static void qmfForwardModulationLP_even(
    HANDLE_QMF_FILTER_BANK anaQmf, /*!< Handle of Qmf Analysis Bank  */
    FIXP_DBL *timeIn,              /*!< Time Signal */
    FIXP_DBL *rSubband)            /*!< Real Output */
{
  int i;
  int L = anaQmf->no_channels;
  int M = L >> 1;
  int scale;
  FIXP_DBL accu;

  const FIXP_DBL *timeInTmp1 = (FIXP_DBL *)&timeIn[3 * M];
  const FIXP_DBL *timeInTmp2 = timeInTmp1;
  FIXP_DBL *rSubbandTmp = rSubband;

  rSubband[0] = timeIn[3 * M] >> 1;

  for (i = M - 1; i != 0; i--) {
    accu = ((*--timeInTmp1) >> 1) + ((*++timeInTmp2) >> 1);
    *++rSubbandTmp = accu;
  }

  timeInTmp1 = &timeIn[2 * M];
  timeInTmp2 = &timeIn[0];
  rSubbandTmp = &rSubband[M];

  for (i = L - M; i != 0; i--) {
    accu = ((*timeInTmp1--) >> 1) - ((*timeInTmp2++) >> 1);
    *rSubbandTmp++ = accu;
  }

  dct_III(rSubband, timeIn, L, &scale);
}

#if !defined(FUNCTION_qmfForwardModulationLP_odd)
static void qmfForwardModulationLP_odd(
    HANDLE_QMF_FILTER_BANK anaQmf, /*!< Handle of Qmf Analysis Bank  */
    const FIXP_DBL *timeIn,        /*!< Time Signal */
    FIXP_DBL *rSubband)            /*!< Real Output */
{
  int i;
  int L = anaQmf->no_channels;
  int M = L >> 1;
  int shift = (anaQmf->no_channels >> 6) + 1;

  for (i = 0; i < M; i++) {
    rSubband[M + i] = (timeIn[L - 1 - i] >> 1) - (timeIn[i] >> shift);
    rSubband[M - 1 - i] =
        (timeIn[L + i] >> 1) + (timeIn[2 * L - 1 - i] >> shift);
  }

  dct_IV(rSubband, L, &shift);
}
#endif /* !defined(FUNCTION_qmfForwardModulationLP_odd) */

/*!
 *
 * \brief Perform complex-valued forward modulation of the time domain
 *        data of timeIn and stores the real part of the subband
 *        samples in rSubband, and the imaginary part in iSubband
 *
 *
 */
#if !defined(FUNCTION_qmfForwardModulationHQ)
static void qmfForwardModulationHQ(
    HANDLE_QMF_FILTER_BANK anaQmf,   /*!< Handle of Qmf Analysis Bank  */
    const FIXP_DBL *RESTRICT timeIn, /*!< Time Signal */
    FIXP_DBL *RESTRICT rSubband,     /*!< Real Output */
    FIXP_DBL *RESTRICT iSubband      /*!< Imaginary Output */
) {
  int i;
  int L = anaQmf->no_channels;
  int L2 = L << 1;
  int shift = 0;

  /* Time advance by one sample, which is equivalent to the complex
     rotation at the end of the analysis. Works only for STD mode. */
  if ((L == 64) && !(anaQmf->flags & (QMF_FLAG_CLDFB | QMF_FLAG_MPSLDFB))) {
    FIXP_DBL x, y;

    /*rSubband[0] = u[1] + u[0]*/
    /*iSubband[0] = u[1] - u[0]*/
    x = timeIn[1] >> 1;
    y = timeIn[0];
    rSubband[0] = x + (y >> 1);
    iSubband[0] = x - (y >> 1);

    /*rSubband[n] = u[n+1] - u[2M-n], n=1,...,M-1*/
    /*iSubband[n] = u[n+1] + u[2M-n], n=1,...,M-1*/
    for (i = 1; i < L; i++) {
      x = timeIn[i + 1] >> 1; /*u[n+1]  */
      y = timeIn[L2 - i];     /*u[2M-n] */
      rSubband[i] = x - (y >> 1);
      iSubband[i] = x + (y >> 1);
    }
  } else {
    for (i = 0; i < L; i += 2) {
      FIXP_DBL x0, x1, y0, y1;

      x0 = timeIn[i + 0] >> 1;
      x1 = timeIn[i + 1] >> 1;
      y0 = timeIn[L2 - 1 - i];
      y1 = timeIn[L2 - 2 - i];

      rSubband[i + 0] = x0 - (y0 >> 1);
      rSubband[i + 1] = x1 - (y1 >> 1);
      iSubband[i + 0] = x0 + (y0 >> 1);
      iSubband[i + 1] = x1 + (y1 >> 1);
    }
  }

  dct_IV(rSubband, L, &shift);
  dst_IV(iSubband, L, &shift);

  /* Do the complex rotation except for the case of 64 bands (in STD mode). */
  if ((L != 64) || (anaQmf->flags & (QMF_FLAG_CLDFB | QMF_FLAG_MPSLDFB))) {
    if (anaQmf->flags & QMF_FLAG_MPSLDFB_OPTIMIZE_MODULATION) {
      FIXP_DBL iBand;
      for (i = 0; i < fMin(anaQmf->lsb, L); i += 2) {
        iBand = rSubband[i];
        rSubband[i] = -iSubband[i];
        iSubband[i] = iBand;

        iBand = -rSubband[i + 1];
        rSubband[i + 1] = iSubband[i + 1];
        iSubband[i + 1] = iBand;
      }
    } else {
      const FIXP_QTW *sbr_t_cos;
      const FIXP_QTW *sbr_t_sin;
      const int len = L; /* was len = fMin(anaQmf->lsb, L) but in case of USAC
                            the signal above lsb is actually needed in some
                            cases (HBE?) */
      sbr_t_cos = anaQmf->t_cos;
      sbr_t_sin = anaQmf->t_sin;

      for (i = 0; i < len; i++) {
        cplxMult(&iSubband[i], &rSubband[i], iSubband[i], rSubband[i],
                 sbr_t_cos[i], sbr_t_sin[i]);
      }
    }
  }
}
#endif /* FUNCTION_qmfForwardModulationHQ */

/*
 * \brief Perform one QMF slot analysis of the time domain data of timeIn
 *        with specified stride and stores the real part of the subband
 *        samples in rSubband, and the imaginary part in iSubband
 *
 *        Note: anaQmf->lsb can be greater than anaQmf->no_channels in case
 *        of implicit resampling (USAC with reduced 3/4 core frame length).
 */
#if (SAMPLE_BITS != DFRACT_BITS) && (QAS_BITS == DFRACT_BITS)
void qmfAnalysisFilteringSlot(
    HANDLE_QMF_FILTER_BANK anaQmf, /*!< Handle of Qmf Synthesis Bank  */
    FIXP_DBL *qmfReal,             /*!< Low and High band, real */
    FIXP_DBL *qmfImag,             /*!< Low and High band, imag */
    const LONG *RESTRICT timeIn,   /*!< Pointer to input */
    const int stride,              /*!< stride factor of input */
    FIXP_DBL *pWorkBuffer          /*!< pointer to temporal working buffer */
) {
  int offset = anaQmf->no_channels * (QMF_NO_POLY * 2 - 1);
  /*
    Feed time signal into oldest anaQmf->no_channels states
  */
  {
    FIXP_DBL *FilterStatesAnaTmp = ((FIXP_DBL *)anaQmf->FilterStates) + offset;

    /* Feed and scale actual time in slot */
    for (int i = anaQmf->no_channels >> 1; i != 0; i--) {
      /* Place INT_PCM value left aligned in scaledTimeIn */

      *FilterStatesAnaTmp++ = (FIXP_QAS)*timeIn;
      timeIn += stride;
      *FilterStatesAnaTmp++ = (FIXP_QAS)*timeIn;
      timeIn += stride;
    }
  }

  if (anaQmf->flags & QMF_FLAG_NONSYMMETRIC) {
    qmfAnaPrototypeFirSlot_NonSymmetric(pWorkBuffer, anaQmf->no_channels,
                                        anaQmf->p_filter, anaQmf->p_stride,
                                        (FIXP_QAS *)anaQmf->FilterStates);
  } else {
    qmfAnaPrototypeFirSlot(pWorkBuffer, anaQmf->no_channels, anaQmf->p_filter,
                           anaQmf->p_stride, (FIXP_QAS *)anaQmf->FilterStates);
  }

  if (anaQmf->flags & QMF_FLAG_LP) {
    if (anaQmf->flags & QMF_FLAG_CLDFB)
      qmfForwardModulationLP_odd(anaQmf, pWorkBuffer, qmfReal);
    else
      qmfForwardModulationLP_even(anaQmf, pWorkBuffer, qmfReal);

  } else {
    qmfForwardModulationHQ(anaQmf, pWorkBuffer, qmfReal, qmfImag);
  }
  /*
    Shift filter states

    Should be realized with modulo adressing on a DSP instead of a true buffer
    shift
  */
  FDKmemmove(anaQmf->FilterStates,
             (FIXP_QAS *)anaQmf->FilterStates + anaQmf->no_channels,
             offset * sizeof(FIXP_QAS));
}
#endif

void qmfAnalysisFilteringSlot(
    HANDLE_QMF_FILTER_BANK anaQmf,  /*!< Handle of Qmf Synthesis Bank  */
    FIXP_DBL *qmfReal,              /*!< Low and High band, real */
    FIXP_DBL *qmfImag,              /*!< Low and High band, imag */
    const INT_PCM *RESTRICT timeIn, /*!< Pointer to input */
    const int stride,               /*!< stride factor of input */
    FIXP_DBL *pWorkBuffer           /*!< pointer to temporal working buffer */
) {
  int offset = anaQmf->no_channels * (QMF_NO_POLY * 2 - 1);
  /*
    Feed time signal into oldest anaQmf->no_channels states
  */
  {
    FIXP_QAS *FilterStatesAnaTmp = ((FIXP_QAS *)anaQmf->FilterStates) + offset;

    /* Feed and scale actual time in slot */
    for (int i = anaQmf->no_channels >> 1; i != 0; i--) {
    /* Place INT_PCM value left aligned in scaledTimeIn */
#if (QAS_BITS == SAMPLE_BITS)
      *FilterStatesAnaTmp++ = (FIXP_QAS)*timeIn;
      timeIn += stride;
      *FilterStatesAnaTmp++ = (FIXP_QAS)*timeIn;
      timeIn += stride;
#elif (QAS_BITS > SAMPLE_BITS)
      *FilterStatesAnaTmp++ = ((FIXP_QAS)*timeIn) << (QAS_BITS - SAMPLE_BITS);
      timeIn += stride;
      *FilterStatesAnaTmp++ = ((FIXP_QAS)*timeIn) << (QAS_BITS - SAMPLE_BITS);
      timeIn += stride;
#else
      *FilterStatesAnaTmp++ = (FIXP_QAS)((*timeIn) >> (SAMPLE_BITS - QAS_BITS));
      timeIn += stride;
      *FilterStatesAnaTmp++ = (FIXP_QAS)((*timeIn) >> (SAMPLE_BITS - QAS_BITS));
      timeIn += stride;
#endif
    }
  }

  if (anaQmf->flags & QMF_FLAG_NONSYMMETRIC) {
    qmfAnaPrototypeFirSlot_NonSymmetric(pWorkBuffer, anaQmf->no_channels,
                                        anaQmf->p_filter, anaQmf->p_stride,
                                        (FIXP_QAS *)anaQmf->FilterStates);
  } else {
    qmfAnaPrototypeFirSlot(pWorkBuffer, anaQmf->no_channels, anaQmf->p_filter,
                           anaQmf->p_stride, (FIXP_QAS *)anaQmf->FilterStates);
  }

  if (anaQmf->flags & QMF_FLAG_LP) {
    if (anaQmf->flags & QMF_FLAG_CLDFB)
      qmfForwardModulationLP_odd(anaQmf, pWorkBuffer, qmfReal);
    else
      qmfForwardModulationLP_even(anaQmf, pWorkBuffer, qmfReal);

  } else {
    qmfForwardModulationHQ(anaQmf, pWorkBuffer, qmfReal, qmfImag);
  }
  /*
    Shift filter states

    Should be realized with modulo adressing on a DSP instead of a true buffer
    shift
  */
  FDKmemmove(anaQmf->FilterStates,
             (FIXP_QAS *)anaQmf->FilterStates + anaQmf->no_channels,
             offset * sizeof(FIXP_QAS));
}

/*!
 *
 * \brief Perform complex-valued subband filtering of the time domain
 *        data of timeIn and stores the real part of the subband
 *        samples in rAnalysis, and the imaginary part in iAnalysis
 * The qmf coefficient table is symmetric. The symmetry is expoited by
 * shrinking the coefficient table to half the size. The addressing mode
 * takes care of the symmetries.
 *
 *
 * \sa PolyphaseFiltering
 */
#if (SAMPLE_BITS != DFRACT_BITS) && (QAS_BITS == DFRACT_BITS)
void qmfAnalysisFiltering(
    HANDLE_QMF_FILTER_BANK anaQmf, /*!< Handle of Qmf Analysis Bank */
    FIXP_DBL **qmfReal,            /*!< Pointer to real subband slots */
    FIXP_DBL **qmfImag,            /*!< Pointer to imag subband slots */
    QMF_SCALE_FACTOR *scaleFactor, const LONG *timeIn, /*!< Time signal */
    const int timeIn_e, const int stride,
    FIXP_DBL *pWorkBuffer /*!< pointer to temporal working buffer */
) {
  int i;
  int no_channels = anaQmf->no_channels;

  scaleFactor->lb_scale =
      -ALGORITHMIC_SCALING_IN_ANALYSIS_FILTERBANK - timeIn_e;
  scaleFactor->lb_scale -= anaQmf->filterScale;

  for (i = 0; i < anaQmf->no_col; i++) {
    FIXP_DBL *qmfImagSlot = NULL;

    if (!(anaQmf->flags & QMF_FLAG_LP)) {
      qmfImagSlot = qmfImag[i];
    }

    qmfAnalysisFilteringSlot(anaQmf, qmfReal[i], qmfImagSlot, timeIn, stride,
                             pWorkBuffer);

    timeIn += no_channels * stride;

  } /* no_col loop  i  */
}
#endif

void qmfAnalysisFiltering(
    HANDLE_QMF_FILTER_BANK anaQmf, /*!< Handle of Qmf Analysis Bank */
    FIXP_DBL **qmfReal,            /*!< Pointer to real subband slots */
    FIXP_DBL **qmfImag,            /*!< Pointer to imag subband slots */
    QMF_SCALE_FACTOR *scaleFactor, const INT_PCM *timeIn, /*!< Time signal */
    const int timeIn_e, const int stride,
    FIXP_DBL *pWorkBuffer /*!< pointer to temporal working buffer */
) {
  int i;
  int no_channels = anaQmf->no_channels;

  scaleFactor->lb_scale =
      -ALGORITHMIC_SCALING_IN_ANALYSIS_FILTERBANK - timeIn_e;
  scaleFactor->lb_scale -= anaQmf->filterScale;

  for (i = 0; i < anaQmf->no_col; i++) {
    FIXP_DBL *qmfImagSlot = NULL;

    if (!(anaQmf->flags & QMF_FLAG_LP)) {
      qmfImagSlot = qmfImag[i];
    }

    qmfAnalysisFilteringSlot(anaQmf, qmfReal[i], qmfImagSlot, timeIn, stride,
                             pWorkBuffer);

    timeIn += no_channels * stride;

  } /* no_col loop  i  */
}

/*!
 *
 * \brief Perform low power inverse modulation of the subband
 *        samples stored in rSubband (real part) and iSubband (imaginary
 *        part) and stores the result in pWorkBuffer.
 *
 */
inline static void qmfInverseModulationLP_even(
    HANDLE_QMF_FILTER_BANK synQmf, /*!< Handle of Qmf Synthesis Bank  */
    const FIXP_DBL *qmfReal, /*!< Pointer to qmf real subband slot (input) */
    const int scaleFactorLowBand,  /*!< Scalefactor for Low band */
    const int scaleFactorHighBand, /*!< Scalefactor for High band */
    FIXP_DBL *pTimeOut             /*!< Pointer to qmf subband slot (output)*/
) {
  int i;
  int L = synQmf->no_channels;
  int M = L >> 1;
  int scale;
  FIXP_DBL tmp;
  FIXP_DBL *RESTRICT tReal = pTimeOut;
  FIXP_DBL *RESTRICT tImag = pTimeOut + L;

  /* Move input to output vector with offset */
  scaleValues(&tReal[0], &qmfReal[0], synQmf->lsb, (int)scaleFactorLowBand);
  scaleValues(&tReal[0 + synQmf->lsb], &qmfReal[0 + synQmf->lsb],
              synQmf->usb - synQmf->lsb, (int)scaleFactorHighBand);
  FDKmemclear(&tReal[0 + synQmf->usb], (L - synQmf->usb) * sizeof(FIXP_DBL));

  /* Dct type-2 transform */
  dct_II(tReal, tImag, L, &scale);

  /* Expand output and replace inplace the output buffers */
  tImag[0] = tReal[M];
  tImag[M] = (FIXP_DBL)0;
  tmp = tReal[0];
  tReal[0] = tReal[M];
  tReal[M] = tmp;

  for (i = 1; i < M / 2; i++) {
    /* Imag */
    tmp = tReal[L - i];
    tImag[M - i] = tmp;
    tImag[i + M] = -tmp;

    tmp = tReal[M + i];
    tImag[i] = tmp;
    tImag[L - i] = -tmp;

    /* Real */
    tReal[M + i] = tReal[i];
    tReal[L - i] = tReal[M - i];
    tmp = tReal[i];
    tReal[i] = tReal[M - i];
    tReal[M - i] = tmp;
  }
  /* Remaining odd terms */
  tmp = tReal[M + M / 2];
  tImag[M / 2] = tmp;
  tImag[M / 2 + M] = -tmp;

  tReal[M + M / 2] = tReal[M / 2];
}

inline static void qmfInverseModulationLP_odd(
    HANDLE_QMF_FILTER_BANK synQmf, /*!< Handle of Qmf Synthesis Bank  */
    const FIXP_DBL *qmfReal, /*!< Pointer to qmf real subband slot (input) */
    const int scaleFactorLowBand,  /*!< Scalefactor for Low band */
    const int scaleFactorHighBand, /*!< Scalefactor for High band */
    FIXP_DBL *pTimeOut             /*!< Pointer to qmf subband slot (output)*/
) {
  int i;
  int L = synQmf->no_channels;
  int M = L >> 1;
  int shift = 0;

  /* Move input to output vector with offset */
  scaleValues(pTimeOut + M, qmfReal, synQmf->lsb, scaleFactorLowBand);
  scaleValues(pTimeOut + M + synQmf->lsb, qmfReal + synQmf->lsb,
              synQmf->usb - synQmf->lsb, scaleFactorHighBand);
  FDKmemclear(pTimeOut + M + synQmf->usb, (L - synQmf->usb) * sizeof(FIXP_DBL));

  dct_IV(pTimeOut + M, L, &shift);
  for (i = 0; i < M; i++) {
    pTimeOut[i] = pTimeOut[L - 1 - i];
    pTimeOut[2 * L - 1 - i] = -pTimeOut[L + i];
  }
}

#ifndef FUNCTION_qmfInverseModulationHQ
/*!
 *
 * \brief Perform complex-valued inverse modulation of the subband
 *        samples stored in rSubband (real part) and iSubband (imaginary
 *        part) and stores the result in pWorkBuffer.
 *
 */
inline static void qmfInverseModulationHQ(
    HANDLE_QMF_FILTER_BANK synQmf, /*!< Handle of Qmf Synthesis Bank     */
    const FIXP_DBL *qmfReal,       /*!< Pointer to qmf real subband slot */
    const FIXP_DBL *qmfImag,       /*!< Pointer to qmf imag subband slot */
    const int scaleFactorLowBand,  /*!< Scalefactor for Low band         */
    const int scaleFactorHighBand, /*!< Scalefactor for High band        */
    FIXP_DBL *pWorkBuffer          /*!< WorkBuffer (output)              */
) {
  int i;
  int L = synQmf->no_channels;
  int M = L >> 1;
  int shift = 0;
  FIXP_DBL *RESTRICT tReal = pWorkBuffer;
  FIXP_DBL *RESTRICT tImag = pWorkBuffer + L;

  if (synQmf->flags & QMF_FLAG_CLDFB) {
    for (i = 0; i < synQmf->lsb; i++) {
      cplxMult(&tImag[i], &tReal[i], scaleValue(qmfImag[i], scaleFactorLowBand),
               scaleValue(qmfReal[i], scaleFactorLowBand), synQmf->t_cos[i],
               synQmf->t_sin[i]);
    }
    for (; i < synQmf->usb; i++) {
      cplxMult(&tImag[i], &tReal[i],
               scaleValue(qmfImag[i], scaleFactorHighBand),
               scaleValue(qmfReal[i], scaleFactorHighBand), synQmf->t_cos[i],
               synQmf->t_sin[i]);
    }
  }

  if ((synQmf->flags & QMF_FLAG_CLDFB) == 0) {
    scaleValues(&tReal[0], &qmfReal[0], synQmf->lsb, (int)scaleFactorLowBand);
    scaleValues(&tReal[0 + synQmf->lsb], &qmfReal[0 + synQmf->lsb],
                synQmf->usb - synQmf->lsb, (int)scaleFactorHighBand);
    scaleValues(&tImag[0], &qmfImag[0], synQmf->lsb, (int)scaleFactorLowBand);
    scaleValues(&tImag[0 + synQmf->lsb], &qmfImag[0 + synQmf->lsb],
                synQmf->usb - synQmf->lsb, (int)scaleFactorHighBand);
  }

  FDKmemclear(&tReal[synQmf->usb],
              (synQmf->no_channels - synQmf->usb) * sizeof(FIXP_DBL));
  FDKmemclear(&tImag[synQmf->usb],
              (synQmf->no_channels - synQmf->usb) * sizeof(FIXP_DBL));

  dct_IV(tReal, L, &shift);
  dst_IV(tImag, L, &shift);

  if (synQmf->flags & QMF_FLAG_CLDFB) {
    for (i = 0; i < M; i++) {
      FIXP_DBL r1, i1, r2, i2;
      r1 = tReal[i];
      i2 = tImag[L - 1 - i];
      r2 = tReal[L - i - 1];
      i1 = tImag[i];

      tReal[i] = (r1 - i1) >> 1;
      tImag[L - 1 - i] = -(r1 + i1) >> 1;
      tReal[L - i - 1] = (r2 - i2) >> 1;
      tImag[i] = -(r2 + i2) >> 1;
    }
  } else {
    /* The array accesses are negative to compensate the missing minus sign in
     * the low and hi band gain. */
    /* 26 cycles on ARM926 */
    for (i = 0; i < M; i++) {
      FIXP_DBL r1, i1, r2, i2;
      r1 = -tReal[i];
      i2 = -tImag[L - 1 - i];
      r2 = -tReal[L - i - 1];
      i1 = -tImag[i];

      tReal[i] = (r1 - i1) >> 1;
      tImag[L - 1 - i] = -(r1 + i1) >> 1;
      tReal[L - i - 1] = (r2 - i2) >> 1;
      tImag[i] = -(r2 + i2) >> 1;
    }
  }
}
#endif /* #ifndef FUNCTION_qmfInverseModulationHQ */

/*!
 *
 * \brief Create QMF filter bank instance
 *
 * \return 0 if successful
 *
 */
static int qmfInitFilterBank(
    HANDLE_QMF_FILTER_BANK h_Qmf, /*!< Handle to return */
    void *pFilterStates,          /*!< Handle to filter states */
    int noCols,                   /*!< Number of timeslots per frame */
    int lsb,                      /*!< Lower end of QMF frequency range */
    int usb,                      /*!< Upper end of QMF frequency range */
    int no_channels,              /*!< Number of channels (bands) */
    UINT flags,                   /*!< flags */
    int synflag)                  /*!< 1: synthesis; 0: analysis */
{
  FDKmemclear(h_Qmf, sizeof(QMF_FILTER_BANK));

  if (flags & QMF_FLAG_MPSLDFB) {
    flags |= QMF_FLAG_NONSYMMETRIC;
    flags |= QMF_FLAG_MPSLDFB_OPTIMIZE_MODULATION;

    h_Qmf->t_cos = NULL;
    h_Qmf->t_sin = NULL;
    h_Qmf->filterScale = QMF_MPSLDFB_PFT_SCALE;
    h_Qmf->p_stride = 1;

    switch (no_channels) {
      case 64:
        h_Qmf->p_filter = qmf_mpsldfb_640;
        h_Qmf->FilterSize = 640;
        break;
      case 32:
        h_Qmf->p_filter = qmf_mpsldfb_320;
        h_Qmf->FilterSize = 320;
        break;
      default:
        return -1;
    }
  }

  if (!(flags & QMF_FLAG_MPSLDFB) && (flags & QMF_FLAG_CLDFB)) {
    flags |= QMF_FLAG_NONSYMMETRIC;
    h_Qmf->filterScale = QMF_CLDFB_PFT_SCALE;

    h_Qmf->p_stride = 1;
    switch (no_channels) {
      case 64:
        h_Qmf->t_cos = qmf_phaseshift_cos64_cldfb;
        h_Qmf->t_sin = qmf_phaseshift_sin64_cldfb;
        h_Qmf->p_filter = qmf_cldfb_640;
        h_Qmf->FilterSize = 640;
        break;
      case 32:
        h_Qmf->t_cos = (synflag) ? qmf_phaseshift_cos32_cldfb_syn
                                 : qmf_phaseshift_cos32_cldfb_ana;
        h_Qmf->t_sin = qmf_phaseshift_sin32_cldfb;
        h_Qmf->p_filter = qmf_cldfb_320;
        h_Qmf->FilterSize = 320;
        break;
      case 16:
        h_Qmf->t_cos = (synflag) ? qmf_phaseshift_cos16_cldfb_syn
                                 : qmf_phaseshift_cos16_cldfb_ana;
        h_Qmf->t_sin = qmf_phaseshift_sin16_cldfb;
        h_Qmf->p_filter = qmf_cldfb_160;
        h_Qmf->FilterSize = 160;
        break;
      case 8:
        h_Qmf->t_cos = (synflag) ? qmf_phaseshift_cos8_cldfb_syn
                                 : qmf_phaseshift_cos8_cldfb_ana;
        h_Qmf->t_sin = qmf_phaseshift_sin8_cldfb;
        h_Qmf->p_filter = qmf_cldfb_80;
        h_Qmf->FilterSize = 80;
        break;
      default:
        return -1;
    }
  }

  if (!(flags & QMF_FLAG_MPSLDFB) && ((flags & QMF_FLAG_CLDFB) == 0)) {
    switch (no_channels) {
      case 64:
        h_Qmf->p_filter = qmf_pfilt640;
        h_Qmf->t_cos = qmf_phaseshift_cos64;
        h_Qmf->t_sin = qmf_phaseshift_sin64;
        h_Qmf->p_stride = 1;
        h_Qmf->FilterSize = 640;
        h_Qmf->filterScale = 0;
        break;
      case 40:
        if (synflag) {
          break;
        } else {
          h_Qmf->p_filter = qmf_pfilt400; /* Scaling factor 0.8 */
          h_Qmf->t_cos = qmf_phaseshift_cos40;
          h_Qmf->t_sin = qmf_phaseshift_sin40;
          h_Qmf->filterScale = 1;
          h_Qmf->p_stride = 1;
          h_Qmf->FilterSize = no_channels * 10;
        }
        break;
      case 32:
        h_Qmf->p_filter = qmf_pfilt640;
        if (flags & QMF_FLAG_DOWNSAMPLED) {
          h_Qmf->t_cos = qmf_phaseshift_cos_downsamp32;
          h_Qmf->t_sin = qmf_phaseshift_sin_downsamp32;
        } else {
          h_Qmf->t_cos = qmf_phaseshift_cos32;
          h_Qmf->t_sin = qmf_phaseshift_sin32;
        }
        h_Qmf->p_stride = 2;
        h_Qmf->FilterSize = 640;
        h_Qmf->filterScale = 0;
        break;
      case 20:
        h_Qmf->p_filter = qmf_pfilt200;
        h_Qmf->p_stride = 1;
        h_Qmf->FilterSize = 200;
        h_Qmf->filterScale = 0;
        break;
      case 12:
        h_Qmf->p_filter = qmf_pfilt120;
        h_Qmf->p_stride = 1;
        h_Qmf->FilterSize = 120;
        h_Qmf->filterScale = 0;
        break;
      case 8:
        h_Qmf->p_filter = qmf_pfilt640;
        h_Qmf->p_stride = 8;
        h_Qmf->FilterSize = 640;
        h_Qmf->filterScale = 0;
        break;
      case 16:
        h_Qmf->p_filter = qmf_pfilt640;
        h_Qmf->t_cos = qmf_phaseshift_cos16;
        h_Qmf->t_sin = qmf_phaseshift_sin16;
        h_Qmf->p_stride = 4;
        h_Qmf->FilterSize = 640;
        h_Qmf->filterScale = 0;
        break;
      case 24:
        h_Qmf->p_filter = qmf_pfilt240;
        h_Qmf->t_cos = qmf_phaseshift_cos24;
        h_Qmf->t_sin = qmf_phaseshift_sin24;
        h_Qmf->p_stride = 1;
        h_Qmf->FilterSize = 240;
        h_Qmf->filterScale = 1;
        break;
      default:
        return -1;
    }
  }

  h_Qmf->synScalefactor = h_Qmf->filterScale;
  // DCT|DST dependency
  switch (no_channels) {
    case 128:
      h_Qmf->synScalefactor += ALGORITHMIC_SCALING_IN_SYNTHESIS_FILTERBANK + 1;
      break;
    case 40: {
      h_Qmf->synScalefactor += ALGORITHMIC_SCALING_IN_SYNTHESIS_FILTERBANK - 1;
    } break;
    case 64:
      h_Qmf->synScalefactor += ALGORITHMIC_SCALING_IN_SYNTHESIS_FILTERBANK;
      break;
    case 8:
      h_Qmf->synScalefactor += ALGORITHMIC_SCALING_IN_SYNTHESIS_FILTERBANK - 3;
      break;
    case 12:
      h_Qmf->synScalefactor += ALGORITHMIC_SCALING_IN_SYNTHESIS_FILTERBANK;
      break;
    case 20:
      h_Qmf->synScalefactor += ALGORITHMIC_SCALING_IN_SYNTHESIS_FILTERBANK + 1;
      break;
    case 32:
      h_Qmf->synScalefactor += ALGORITHMIC_SCALING_IN_SYNTHESIS_FILTERBANK - 1;
      break;
    case 16:
      h_Qmf->synScalefactor += ALGORITHMIC_SCALING_IN_SYNTHESIS_FILTERBANK - 2;
      break;
    case 24:
      h_Qmf->synScalefactor += ALGORITHMIC_SCALING_IN_SYNTHESIS_FILTERBANK - 1;
      break;
    default:
      return -1;
  }

  h_Qmf->flags = flags;

  h_Qmf->no_channels = no_channels;
  h_Qmf->no_col = noCols;

  h_Qmf->lsb = fMin(lsb, h_Qmf->no_channels);
  h_Qmf->usb = synflag
                   ? fMin(usb, h_Qmf->no_channels)
                   : usb; /* was: h_Qmf->usb = fMin(usb, h_Qmf->no_channels); */

  h_Qmf->FilterStates = (void *)pFilterStates;

  h_Qmf->outScalefactor =
      (ALGORITHMIC_SCALING_IN_ANALYSIS_FILTERBANK + h_Qmf->filterScale) +
      h_Qmf->synScalefactor;

  h_Qmf->outGain_m =
      (FIXP_DBL)0x80000000; /* default init value will be not applied */
  h_Qmf->outGain_e = 0;

  return (0);
}

/*!
 *
 * \brief Adjust synthesis qmf filter states
 *
 * \return void
 *
 */
static inline void qmfAdaptFilterStates(
    HANDLE_QMF_FILTER_BANK synQmf, /*!< Handle of Qmf Filter Bank */
    int scaleFactorDiff)           /*!< Scale factor difference to be applied */
{
  if (synQmf == NULL || synQmf->FilterStates == NULL) {
    return;
  }
  if (scaleFactorDiff > 0) {
    scaleValuesSaturate((FIXP_QSS *)synQmf->FilterStates,
                        synQmf->no_channels * (QMF_NO_POLY * 2 - 1),
                        scaleFactorDiff);
  } else {
    scaleValues((FIXP_QSS *)synQmf->FilterStates,
                synQmf->no_channels * (QMF_NO_POLY * 2 - 1), scaleFactorDiff);
  }
}

/*!
 *
 * \brief Create QMF filter bank instance
 *
 *
 * \return 0 if succesful
 *
 */
int qmfInitAnalysisFilterBank(
    HANDLE_QMF_FILTER_BANK h_Qmf, /*!< Returns handle */
    FIXP_QAS *pFilterStates,      /*!< Handle to filter states */
    int noCols,                   /*!< Number of timeslots per frame */
    int lsb,                      /*!< lower end of QMF */
    int usb,                      /*!< upper end of QMF */
    int no_channels,              /*!< Number of channels (bands) */
    int flags)                    /*!< Low Power flag */
{
  int err = qmfInitFilterBank(h_Qmf, pFilterStates, noCols, lsb, usb,
                              no_channels, flags, 0);
  if (!(flags & QMF_FLAG_KEEP_STATES) && (h_Qmf->FilterStates != NULL)) {
    FDKmemclear(h_Qmf->FilterStates,
                (2 * QMF_NO_POLY - 1) * h_Qmf->no_channels * sizeof(FIXP_QAS));
  }

  FDK_ASSERT(h_Qmf->no_channels >= h_Qmf->lsb);

  return err;
}

/*!
 *
 * \brief Create QMF filter bank instance
 *
 *
 * \return 0 if succesful
 *
 */
int qmfInitSynthesisFilterBank(
    HANDLE_QMF_FILTER_BANK h_Qmf, /*!< Returns handle */
    FIXP_QSS *pFilterStates,      /*!< Handle to filter states */
    int noCols,                   /*!< Number of timeslots per frame */
    int lsb,                      /*!< lower end of QMF */
    int usb,                      /*!< upper end of QMF */
    int no_channels,              /*!< Number of channels (bands) */
    int flags)                    /*!< Low Power flag */
{
  int oldOutScale = h_Qmf->outScalefactor;
  int err = qmfInitFilterBank(h_Qmf, pFilterStates, noCols, lsb, usb,
                              no_channels, flags, 1);
  if (h_Qmf->FilterStates != NULL) {
    if (!(flags & QMF_FLAG_KEEP_STATES)) {
      FDKmemclear(
          h_Qmf->FilterStates,
          (2 * QMF_NO_POLY - 1) * h_Qmf->no_channels * sizeof(FIXP_QSS));
    } else {
      qmfAdaptFilterStates(h_Qmf, oldOutScale - h_Qmf->outScalefactor);
    }
  }

  FDK_ASSERT(h_Qmf->no_channels >= h_Qmf->lsb);
  FDK_ASSERT(h_Qmf->no_channels >= h_Qmf->usb);

  return err;
}

/*!
 *
 * \brief Change scale factor for output data and adjust qmf filter states
 *
 * \return void
 *
 */
void qmfChangeOutScalefactor(
    HANDLE_QMF_FILTER_BANK synQmf, /*!< Handle of Qmf Synthesis Bank */
    int outScalefactor             /*!< New scaling factor for output data */
) {
  if (synQmf == NULL) {
    return;
  }

  /* Add internal filterbank scale */
  outScalefactor +=
      (ALGORITHMIC_SCALING_IN_ANALYSIS_FILTERBANK + synQmf->filterScale) +
      synQmf->synScalefactor;

  /* adjust filter states when scale factor has been changed */
  if (synQmf->outScalefactor != outScalefactor) {
    int diff;

    diff = synQmf->outScalefactor - outScalefactor;

    qmfAdaptFilterStates(synQmf, diff);

    /* save new scale factor */
    synQmf->outScalefactor = outScalefactor;
  }
}

/*!
 *
 * \brief Get scale factor change which was set by qmfChangeOutScalefactor()
 *
 * \return scaleFactor
 *
 */
int qmfGetOutScalefactor(
    HANDLE_QMF_FILTER_BANK synQmf) /*!< Handle of Qmf Synthesis Bank */
{
  int scaleFactor = synQmf->outScalefactor
                        ? (synQmf->outScalefactor -
                           (ALGORITHMIC_SCALING_IN_ANALYSIS_FILTERBANK +
                            synQmf->filterScale + synQmf->synScalefactor))
                        : 0;
  return scaleFactor;
}

/*!
 *
 * \brief Change gain for output data
 *
 * \return void
 *
 */
void qmfChangeOutGain(
    HANDLE_QMF_FILTER_BANK synQmf, /*!< Handle of Qmf Synthesis Bank */
    FIXP_DBL outputGain,           /*!< New gain for output data (mantissa) */
    int outputGainScale            /*!< New gain for output data (exponent) */
) {
  synQmf->outGain_m = outputGain;
  synQmf->outGain_e = outputGainScale;
}

/* When QMF_16IN_32OUT is set, synthesis functions for 16 and 32 bit parallel
 * output is compiled */
#define INT_PCM_QMFOUT INT_PCM
#define SAMPLE_BITS_QMFOUT SAMPLE_BITS
#include "qmf_pcm.h"
