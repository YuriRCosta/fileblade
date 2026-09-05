import QtQuick
import QtTest
import "../../lib/FolderPalette.js" as FolderPalette

TestCase {
  name: "FolderPalette"

  readonly property var gitColors: ({ modified: "#e5c07b", added: "#98c379", renamed: "#61afef", deleted: "#a55555" })

  function hueGap(a, b) {
    var d = Math.abs(FolderPalette.toHsl(a).h - FolderPalette.toHsl(b).h) % 360
    return Math.min(d, 360 - d)
  }

  function test_shifted_keys_leave_every_git_hue() {
    var shifted = {
      red: FolderPalette.distinct("red", "#e06c75"),
      yellow: FolderPalette.distinct("yellow", "#e5c07b"),
      green: FolderPalette.distinct("green", "#98c379")
    }
    var keys = Object.keys(shifted)
    for (var i = 0; i < keys.length; i++) {
      var git = Object.keys(gitColors)
      for (var j = 0; j < git.length; j++) {
        verify(hueGap(shifted[keys[i]], gitColors[git[j]]) >= 15,
               keys[i] + " " + shifted[keys[i]] + " sits on git " + git[j] + " " + gitColors[git[j]])
      }
    }
  }

  function test_theme_colours_shift_the_same_way() {
    var lemon = FolderPalette.distinct("yellow", "#e0af68")
    verify(lemon !== "#e0af68")
    verify(hueGap(lemon, "#e0af68") >= 15 && hueGap(lemon, "#e0af68") <= 21)
  }

  function test_other_keys_and_bad_input_pass_through() {
    compare(FolderPalette.distinct("blue", "#61afef"), "#61afef")
    compare(FolderPalette.distinct("muted", "#abb2bf"), "#abb2bf")
    compare(FolderPalette.distinct("yellow", "not a colour"), "not a colour")
    compare(FolderPalette.distinct("yellow", ""), "")
  }

  function test_user_file_replaces_named_swatches_only() {
    var palette = FolderPalette.parseUser('{"version":1,"folder":{"red":"#F38BA8","blue":"#89b4fa","bogus":"#000000"}}')
    compare(palette, { red: "#f38ba8", blue: "#89b4fa" })
    compare(FolderPalette.parseUser('{"version":1}'), {})
    compare(FolderPalette.parseUser(""), {})
  }

  function test_user_file_rejects_bad_values_whole() {
    var failed = ""
    try { FolderPalette.parseUser('{"folder":{"red":"red"}}') } catch (error) { failed = String(error) }
    verify(failed.indexOf("six-digit hex") >= 0, failed)
    failed = ""
    try { FolderPalette.parseUser('{"folder":[]}') } catch (error) { failed = String(error) }
    verify(failed.indexOf("folder") >= 0, failed)
    failed = ""
    try { FolderPalette.parseUser('[1]') } catch (error) { failed = String(error) }
    verify(failed.indexOf("object") >= 0, failed)
  }

  function test_round_trip_keeps_a_colour() {
    compare(FolderPalette.toHex(FolderPalette.toHsl("#61afef")), "#61afef")
    compare(FolderPalette.toHex(FolderPalette.toHsl("#000000")), "#000000")
  }
}
