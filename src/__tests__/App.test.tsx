import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "@/App";
import { invoke } from "@tauri-apps/api/core";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

beforeEach(() => {
  vi.mocked(invoke).mockResolvedValue({ jobs: [], isRunning: false });
});

describe("App", () => {
  it("renders the greeting", () => {
    render(<App />);
    expect(screen.getByText("Hi,")).toBeInTheDocument();
  });

  it("renders the file drop zone", () => {
    render(<App />);
    expect(screen.getByText("Drop PDF files here")).toBeInTheDocument();
  });

  it("renders the output directory picker", () => {
    render(<App />);
    expect(screen.getByText("Output Directory")).toBeInTheDocument();
  });

  it("shows error toast when starting with no files", async () => {
    render(<App />);
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: /start ocr/i }));
    expect(screen.getByText("No files in queue")).toBeInTheDocument();
  });

  it("shows error toast when starting with no output dir", async () => {
    render(<App />);
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: /start ocr/i }));
    await vi.dynamicImportSettled();
  });
});
