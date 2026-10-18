/**
 * ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
 * Author: SAIL
 * Copyright (c) 2026 SAIL
 * SPDX-License-Identifier: MIT
 */
export const frequencies = ["31", "63", "125", "250", "500", "1k", "2k", "4k", "8k", "16k"];

/** Razer's observed default 10-band curve, used for startup and Reset. */
export const DEFAULT_EQ_GAINS = [1, -5, -3, 0, 1, 1, 1, 1, 1, -3];

/** Preferred default for the speaker THX DRC level. Synapse's UI starts at 10;
 * the device accepts 1..100 and this app defaults to 5 unless the user chose
 * another value (persisted in localStorage). */
export const DEFAULT_NORMALIZATION_LEVEL = 5;
