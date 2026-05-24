import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { FileDropZone } from "@/components/file-dropzone";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

describe("FileDropZone", () => {
  const onFilesAdded = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(invoke).mockResolvedValue({ size: 1024 });
  });

  it("renders drop zone with correct text", () => {
    render(<FileDropZone onFilesAdded={onFilesAdded} />);
    expect(screen.getByText("Drop PDF files here")).toBeInTheDocument();
    expect(screen.getByText("or click to browse")).toBeInTheDocument();
  });

  it("opens file dialog on click", async () => {
    vi.mocked(open).mockResolvedValue(["/path/to/doc.pdf"]);
    render(<FileDropZone onFilesAdded={onFilesAdded} />);
    const user = userEvent.setup();
    await user.click(screen.getByRole("button"));
    expect(open).toHaveBeenCalledWith({
      multiple: true,
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });
  });

  it("ignores non-PDF files", async () => {
    vi.mocked(open).mockResolvedValue(["/path/to/image.png"]);
    render(<FileDropZone onFilesAdded={onFilesAdded} />);
    const user = userEvent.setup();
    await user.click(screen.getByRole("button"));
    expect(onFilesAdded).not.toHaveBeenCalled();
  });

  it("handles dialog cancellation", async () => {
    vi.mocked(open).mockResolvedValue(null);
    render(<FileDropZone onFilesAdded={onFilesAdded} />);
    const user = userEvent.setup();
    await user.click(screen.getByRole("button"));
    expect(onFilesAdded).not.toHaveBeenCalled();
  });
});
