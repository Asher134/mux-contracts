/**
 * Unit tests for MuxBatcherClient binding shape and batcher-specific error mapping.
 */

import { MuxBatcherClient } from "../src/generated/mux-batcher";
import { contractErrorToHttp, ERROR_HTTP_MAP } from "../src/errors";
import { muxBatcherErrorMessage } from "../src/types";

describe("MuxBatcherClient shape", () => {
  it("exposes executeBatch as a function", () => {
    expect(typeof MuxBatcherClient.prototype.executeBatch).toBe("function");
  });

  it("exposes simulateBatch as a function", () => {
    expect(typeof MuxBatcherClient.prototype.simulateBatch).toBe("function");
  });

  it("exposes maxBatchSize as a function", () => {
    expect(typeof MuxBatcherClient.prototype.maxBatchSize).toBe("function");
  });

  it("exposes submitBatch as a function", () => {
    expect(typeof MuxBatcherClient.prototype.submitBatch).toBe("function");
  });

  it("exposes setRegistryMetadata as a function", () => {
    expect(typeof MuxBatcherClient.prototype.setRegistryMetadata).toBe("function");
  });

  it("exposes getRegistryMetadata as a function", () => {
    expect(typeof MuxBatcherClient.prototype.getRegistryMetadata).toBe("function");
  });
});

describe("Batcher error HTTP mapping", () => {
  it("maps BatchTooLarge to 400", () => {
    expect(ERROR_HTTP_MAP.BatchTooLarge).toBe(400);
  });

  it("maps EmptyBatch to 400", () => {
    expect(ERROR_HTTP_MAP.EmptyBatch).toBe(400);
  });

  it("maps RequiredOperationFailed to 500", () => {
    expect(ERROR_HTTP_MAP.RequiredOperationFailed).toBe(500);
  });

  it("maps Unauthorized to 401", () => {
    expect(ERROR_HTTP_MAP.Unauthorized).toBe(401);
  });

  it("maps ReentrancyDetected to 409", () => {
    expect(ERROR_HTTP_MAP.ReentrancyDetected).toBe(409);
  });

  it("maps MetadataAlreadySet to 409", () => {
    expect(ERROR_HTTP_MAP.MetadataAlreadySet).toBe(409);
  });

  it("contractErrorToHttp returns correct shape for batcher errors", () => {
    const r = contractErrorToHttp("BatchTooLarge");
    expect(r.statusCode).toBe(400);
    expect(r.errorType).toBe("BatchTooLarge");
    expect(r.message).toBe("BatchTooLarge");
  });

  it("contractErrorToHttp returns 409 for MetadataAlreadySet", () => {
    const r = contractErrorToHttp("MetadataAlreadySet");
    expect(r.statusCode).toBe(409);
    expect(r.errorType).toBe("MetadataAlreadySet");
  });
});

describe("muxBatcherErrorMessage", () => {
  it("returns a description for EmptyBatch by name", () => {
    expect(muxBatcherErrorMessage("EmptyBatch")).toBe("batch contains no operations");
  });

  it("returns a description for BatchTooLarge by name", () => {
    expect(muxBatcherErrorMessage("BatchTooLarge")).toBe(
      "batch exceeds the maximum operation count"
    );
  });

  it("returns a description for RequiredOperationFailed by name", () => {
    expect(muxBatcherErrorMessage("RequiredOperationFailed")).toBe(
      "a required operation failed; the batch was aborted"
    );
  });

  it("returns a description for Unauthorized by name", () => {
    expect(muxBatcherErrorMessage("Unauthorized")).toBe("caller is not authorized");
  });

  it("returns a description for ReentrancyDetected by name", () => {
    expect(muxBatcherErrorMessage("ReentrancyDetected")).toBe(
      "reentrant call into the batcher detected"
    );
  });

  it("returns a description for MetadataAlreadySet by name", () => {
    expect(muxBatcherErrorMessage("MetadataAlreadySet")).toBe(
      "metadata has already been set for this batcher instance"
    );
  });

  it("resolves error code 6 to MetadataAlreadySet description", () => {
    expect(muxBatcherErrorMessage(6)).toBe(
      "metadata has already been set for this batcher instance"
    );
  });

  it("resolves error code 1 to EmptyBatch description", () => {
    expect(muxBatcherErrorMessage(1)).toBe("batch contains no operations");
  });

  it("resolves error code 2 to BatchTooLarge description", () => {
    expect(muxBatcherErrorMessage(2)).toBe("batch exceeds the maximum operation count");
  });

  it("returns unknown error code for an unrecognised code", () => {
    expect(muxBatcherErrorMessage(999)).toBe("unknown error code");
  });
});
