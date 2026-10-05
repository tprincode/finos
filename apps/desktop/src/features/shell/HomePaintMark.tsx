import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

let marked = false;

/** After Home commits, write `page loaded Home` into the desktop log. */
export function HomePaintMark() {
  useEffect(() => {
    if (marked) return;
    const home = document.querySelector("button[aria-label='Home']");
    const row = document.querySelector(".home-top-row");
    if (!home || !row) return;
    marked = true;
    void invoke("page_loaded", { screen: "Home" });
  }, []);
  return null;
}
