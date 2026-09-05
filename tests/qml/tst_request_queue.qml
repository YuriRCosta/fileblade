import QtQuick
import QtTest
import "../../lib/RequestQueue.js" as RequestQueue

TestCase {
  name: "RequestQueue"

  function test_capacity_reserves_one_background_slot() {
    compare(RequestQueue.maximumConcurrency({ concurrency: 16 }), 16)
    compare(RequestQueue.backgroundCapacity({ concurrency: 16 }), 15)
    compare(RequestQueue.maximumConcurrency({ concurrency: 0 }), 16)
    compare(RequestQueue.backgroundCapacity({ concurrency: 1 }), 1)
  }

  function test_latency_sensitive_reads_are_interactive() {
    verify(RequestQueue.interactive("children-window"))
    verify(RequestQueue.interactive("project-root"))
    verify(RequestQueue.interactive("hover-target"))
    verify(!RequestQueue.interactive("search"))
    verify(!RequestQueue.interactive("git-metadata-batch"))
    verify(RequestQueue.interactive("search", { priority: "interactive" }))
    verify(!RequestQueue.interactive("children-window", { priority: "background" }))
  }

  function test_interactive_work_uses_the_reserved_lane() {
    verify(!RequestQueue.canTransmit(false, 15, { concurrency: 16 }))
    verify(RequestQueue.canTransmit(true, 15, { concurrency: 16 }))
    compare(RequestQueue.nextWaitingIndex([false, false, true], 14, { concurrency: 16 }), 2)
    compare(RequestQueue.nextWaitingIndex([false, false], 15, { concurrency: 16 }), -1)
    compare(RequestQueue.nextWaitingIndex([false, true], 15, { concurrency: 16 }), 1)
    verify(!RequestQueue.canTransmit(true, 16, { concurrency: 16 }))
  }
}
