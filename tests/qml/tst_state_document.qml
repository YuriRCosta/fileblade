import QtQuick
import QtTest
import "../../lib/StateDocument.js" as StateDocument

TestCase {
  name: "StateDocument"

  function test_successful_read_applies_and_stays_writable() {
    var plan = StateDocument.hydrationPlan({ ok: true, text: "{\"favorites\": []}" })
    verify(plan.apply)
    verify(plan.writable)
    compare(plan.text, "{\"favorites\": []}")
    verify(!plan.retry)
    compare(plan.warning, "")
  }

  function test_missing_file_applies_defaults_and_stays_writable() {
    var plan = StateDocument.hydrationPlan({ ok: true, text: "" })
    verify(plan.apply)
    verify(plan.writable)
    compare(plan.text, "")
  }

  function test_quarantined_file_applies_defaults_and_warns() {
    var plan = StateDocument.hydrationPlan({ ok: true, text: "", quarantined: "/s/state.json.corrupt-1", error: "invalid JSON document" })
    verify(plan.apply)
    verify(plan.writable)
    verify(plan.warning.indexOf("/s/state.json.corrupt-1") >= 0)
  }

  function test_failed_read_never_applies_or_writes_and_retries() {
    var plan = StateDocument.hydrationPlan({ ok: false, error: "Backend exited with 127" })
    verify(!plan.apply)
    verify(!plan.writable)
    verify(plan.retry)
    verify(plan.warning.indexOf("Backend exited with 127") >= 0)
  }

  function test_cancelled_read_does_not_retry() {
    var plan = StateDocument.hydrationPlan({ ok: false, cancelled: true, error: "request cancelled" })
    verify(!plan.writable)
    verify(!plan.retry)
  }

  function test_empty_response_is_read_only() {
    var plan = StateDocument.hydrationPlan(null)
    verify(!plan.apply)
    verify(!plan.writable)
    verify(plan.retry)
  }
}
