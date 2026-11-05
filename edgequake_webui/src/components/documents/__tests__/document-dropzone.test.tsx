/**
 * @vitest-environment jsdom
 */
import { DEFAULT_VISION_EXTRACT_DRAFT } from "@/components/settings/vision-extract-controls";
import i18n from "@/lib/i18n";
import { resetTextOnlyNoticeDismissals } from "@/lib/documents/text-only-notice-dismiss";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  DocumentDropzone,
  type DocumentDropzoneProps,
} from "../document-dropzone";

vi.mock("@/hooks/use-llm-capability-models", () => ({
  useLlmCapabilityModels: () => undefined,
}));
vi.mock("@/hooks/use-providers", () => ({
  useLlmModels: () => ({ data: undefined }),
}));

vi.mock("@/hooks/use-decision-status", () => ({
  useDecisionStatus: () => ({
    data: {
      enabled: true,
      activation: "forced",
      limits: { pack_size: 4, timeout_secs: 600 },
    },
  }),
}));

function installBox(width: number, height: number) {
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(
    () =>
      ({
        width,
        height,
        top: 0,
        left: 0,
        bottom: height,
        right: width,
        x: 0,
        y: 0,
        toJSON() {},
      }) as DOMRect,
  );
  class ResizeObserverStub {
    private readonly cb: ResizeObserverCallback;
    constructor(cb: ResizeObserverCallback) {
      this.cb = cb;
    }
    observe() {
      this.cb([], this as unknown as ResizeObserver);
    }
    unobserve() {}
    disconnect() {}
  }
  vi.stubGlobal("ResizeObserver", ResizeObserverStub);
}

function renderZone(overrides: Partial<DocumentDropzoneProps> = {}) {
  const openFileDialog = vi.fn();
  render(
    <DocumentDropzone
      getRootProps={(props) => ({ ...(props ?? {}) })}
      getInputProps={() => ({})}
      isDragActive={false}
      openFileDialog={openFileDialog}
      pdfParserBackend="default"
      onPdfParserBackendChange={() => {}}
      workspacePdfParserBackend="vision"
      visionExtract={DEFAULT_VISION_EXTRACT_DRAFT}
      onVisionExtractChange={() => {}}
      fill
      {...overrides}
    />,
  );
  return { openFileDialog };
}

describe("DocumentDropzone", () => {
  beforeEach(async () => {
    await i18n.changeLanguage("en");
    localStorage.clear();
    resetTextOnlyNoticeDismissals();
  });

  afterEach(() => {
    cleanup();
    resetTextOnlyNoticeDismissals();
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it("centers an invitation and docks the PDF parser on a tall panel", () => {
    installBox(720, 640);
    const { openFileDialog } = renderZone();

    const zone = screen.getByTestId("document-dropzone");
    expect(zone).toHaveAttribute("data-density", "roomy");
    expect(screen.getByTestId("document-dropzone-title")).toHaveTextContent(
      "Add documents",
    );
    expect(
      screen.getByText("Drop files here, or choose them from your computer."),
    ).toBeInTheDocument();
    expect(screen.getByTestId("document-dropzone-formats")).toHaveTextContent(
      "PDF",
    );
    expect(screen.getByText(/Word and Excel/)).toBeInTheDocument();
    expect(screen.getByText("PDF parser")).toBeInTheDocument();
    expect(screen.getByTestId("vision-settings-panel-trigger")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("document-dropzone-browse"));
    expect(openFileDialog).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByText("PDF parser"));
    expect(openFileDialog).toHaveBeenCalledTimes(1);
  });

  it("keeps a compact prompt when the panel is too short to center", () => {
    installBox(720, 200);
    renderZone();

    expect(screen.getByTestId("document-dropzone")).toHaveAttribute(
      "data-density",
      "compact",
    );
    expect(screen.getByTestId("document-dropzone-browse")).toBeInTheDocument();
    expect(screen.queryByTestId("document-dropzone-formats")).not.toBeInTheDocument();
    expect(screen.getByText("PDF parser")).toBeInTheDocument();
  });

  it("groups the prompt and parser in a tall narrow column", () => {
    installBox(220, 640);
    renderZone();

    const zone = screen.getByTestId("document-dropzone");
    expect(zone).toHaveAttribute("data-fill-layout", "stack");
    expect(zone).toHaveAttribute("data-density", "compact");
    expect(screen.getByTestId("document-dropzone-browse")).toBeInTheDocument();
    expect(screen.queryByTestId("document-dropzone-formats")).not.toBeInTheDocument();
    expect(screen.getByText("PDF parser")).toBeInTheDocument();
  });

  it("uses a single band and no browse button on a short strip", () => {
    installBox(840, 80);
    renderZone();

    const zone = screen.getByTestId("document-dropzone");
    expect(zone).toHaveAttribute("data-fill-layout", "row");
    expect(zone).toHaveAttribute("data-density", "row");
    expect(screen.getByTestId("document-dropzone-title")).toHaveTextContent(
      "Drop files or choose them",
    );
    expect(screen.queryByTestId("document-dropzone-browse")).not.toBeInTheDocument();
  });

  it("stacks a short strip that is too narrow to hold every control on one line", () => {
    installBox(560, 80);
    renderZone();

    expect(screen.getByTestId("document-dropzone")).toHaveAttribute("data-fill-layout", "stack");
  });

  it("replaces the invitation with a release prompt while dragging", () => {
    installBox(720, 640);
    renderZone({ isDragActive: true });

    expect(screen.getByTestId("document-dropzone-title")).toHaveTextContent(
      "Release to upload",
    );
    expect(screen.queryByTestId("document-dropzone-browse")).not.toBeInTheDocument();
    expect(screen.queryByTestId("document-dropzone-formats")).not.toBeInTheDocument();
  });

  it("wraps parser and extraction controls on a narrow dock instead of overlapping", () => {
    installBox(360, 280);
    renderZone({
      quiet: true,
      onExtractionModeChange: () => {},
    });

    const bar = screen.getByTestId("upload-parser-vision-combo");
    expect(bar.className).toMatch(/flex-wrap/);
    expect(bar.className).toMatch(/overflow-hidden/);
    expect(screen.getByTestId("spec038-upload-parser-select").className).toMatch(
      /min-w-0/,
    );
    expect(screen.getByTestId("upload-extraction-mode-select").className).toMatch(
      /min-w-0/,
    );
  });

  it("softens the invitation while processing continues", () => {
    installBox(720, 640);
    renderZone({ quiet: true });

    expect(screen.getByTestId("document-dropzone-title")).toHaveTextContent(
      "Add more files",
    );
    expect(screen.queryByTestId("vision-settings-panel-trigger")).not.toBeInTheDocument();
    expect(screen.getByText("PDF parser")).toBeInTheDocument();
  });

  it("stops advertising Images on a workspace that cannot store them", () => {
    installBox(720, 640);
    renderZone({ acceptsImages: { embeds_images: false } });

    const chips = screen.getByTestId("document-dropzone-formats");
    expect(chips).toHaveTextContent("PDF");
    expect(chips).not.toHaveTextContent("Images");
  });

  it("keeps the Images chip when the capability is unknown", () => {
    installBox(720, 640);
    renderZone({ acceptsImages: null });

    // Absent is not "no": the server decides whether a picture is accepted, so the UI must
    // not remove a format on a guess.
    expect(screen.getByTestId("document-dropzone-formats")).toHaveTextContent("Images");
  });

  it("renders the text-only note above the drop target, not inside it", () => {
    installBox(720, 640);
    renderZone({ acceptsImages: { embeds_images: false } });

    // SPEC-185: the note lives beside the drop target. Inside it, at the short-and-wide
    // layout, it took 897px of a 963px row and squeezed the settings selects to 0px.
    expect(screen.queryByTestId("dropzone-text-only-notice")).not.toBeInTheDocument();
    // The zone itself is untouched: it is still a complete drop target.
    expect(screen.getByTestId("document-dropzone")).toBeInTheDocument();
    expect(screen.getByTestId("document-dropzone-title")).toHaveTextContent(
      "Add documents",
    );
  });
});
