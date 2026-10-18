// ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
// Author: SAIL
// Copyright (c) 2026 SAIL
// SPDX-License-Identifier: MIT
// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    sail_ui_lib::run()
}
