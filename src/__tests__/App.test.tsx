import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "@/App";
import { invoke } from "@tauri-apps/api/core";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    show: vi.fn(),
    setFocus: vi.fn(),
    onDragDropEvent: vi.fn().mockReturnValue(Promise.resolve(vi.fn())),
  }),
}));

vi.mock("@/components/left-panel", async () => {
  const React = await import("react");
  return {
    default: (props: any) =>
      React.createElement("div", null,
        React.createElement("div", null, "Drop PDF files here"),
        React.createElement("div", null, "Output Directory"),
        React.createElement("button", { onClick: () => props.onStart({}) }, "Start OCR Processing"),
      ),
  };
});

vi.mock("@/components/right-panel", async () => {
  const React = await import("react");
  return {
    default: () => React.createElement("div", null, "Right Panel"),
  };
});

beforeEach(() => {
  vi.mocked(invoke).mockResolvedValue({ jobs: [], isRunning: false });
});

describe("App", () => {
  it("renders the greeting", async () => {
    render(<App />);
    expect(await screen.findByText("Hi,")).toBeInTheDocument();
  });

  it("renders the file drop zone", async () => {
    render(<App />);
    expect(await screen.findByText("Drop PDF files here")).toBeInTheDocument();
  });

  it("renders the output directory picker", async () => {
    render(<App />);
    expect(await screen.findByText("Output Directory")).toBeInTheDocument();
  });

  it("shows error toast when starting with no files", async () => {
    render(<App />);
    const user = userEvent.setup();
    expect(await screen.findByRole("button", { name: /start ocr/i }));
    await user.click(screen.getByRole("button", { name: /start ocr/i }));
    expect(await screen.findByText("No files in queue")).toBeInTheDocument();
  });
});
