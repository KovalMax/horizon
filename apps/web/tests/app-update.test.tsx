// @vitest-environment jsdom
import { expect, test, vi } from "vitest";
import { applyAppUpdate } from "../src/features/planner/components/AppUpdateSettings";

test("checks the server even when there is no waiting service worker", async () => {
  const update = vi.fn().mockResolvedValue(undefined);
  await applyAppUpdate({ update } as unknown as ServiceWorkerRegistration);
  expect(update).toHaveBeenCalledOnce();
});

test("asks a waiting worker to activate and waits before allowing reload", async () => {
  const worker = Object.assign(new EventTarget(), {
    state: "installed",
    postMessage: vi.fn(),
  });
  const update = vi.fn();
  const registration = {
    waiting: worker,
    update,
  } as unknown as ServiceWorkerRegistration;
  const finished = vi.fn();
  const result = applyAppUpdate(registration).then(finished);
  expect(worker.postMessage).toHaveBeenCalledWith({
    type: "HORIZON_ACTIVATE_UPDATE",
  });
  expect(update).not.toHaveBeenCalled();
  expect(finished).not.toHaveBeenCalled();
  worker.state = "activated";
  worker.dispatchEvent(new Event("statechange"));
  await result;
  expect(finished).toHaveBeenCalledOnce();
});
