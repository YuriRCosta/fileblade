import QtQuick
import QtTest
import "../../lib/Format.js" as Format

TestCase {
  name: "FormatMetrics"

  function test_standard_option_by_key() {
    var option = Format.metricOption("tokens")
    compare(option.key, "tokens")
    compare(option.label, "Tokens (estimated)")
    compare(option.shortLabel, "TOKENS")
    compare(option.kind, "number")
  }

  function test_override_keeps_standard_fields() {
    var option = Format.metricOption({ key: "tokens", label: "Tokens (descriptions)" })
    compare(option.label, "Tokens (descriptions)")
    compare(option.shortLabel, "TOKENS")
    compare(option.kind, "number")
  }

  function test_unknown_key_derives_fields() {
    var option = Format.metricOption({ key: "transport", shortLabel: "TYPE" })
    compare(option.label, "Transport")
    compare(option.shortLabel, "TYPE")
    compare(option.kind, "text")
    compare(Format.metricOption("fileTokens").kind, "text")
    compare(Format.metricOption({ key: "fileTokens", kind: "number" }).kind, "number")
  }

  function test_options_maps_mixed_specs() {
    var list = Format.metricOptions(["off", "agents", { key: "status" }, "bytes"])
    compare(list.length, 4)
    compare(list[1].kind, "agents")
    compare(list[3].label, "Size")
    compare(Format.metricOptions(null).length, 0)
  }

  function test_token_estimate_rounds_up_four_bytes_per_token() {
    compare(Format.estimateTokens(0), 0)
    compare(Format.estimateTokens(1), 1)
    compare(Format.estimateTokens(4), 1)
    compare(Format.estimateTokens(5), 2)
    compare(Format.estimateTokens(-1), null)
    compare(Format.estimateTokens("abc"), null)
  }
}
