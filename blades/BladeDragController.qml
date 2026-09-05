import QtQuick

Item {
  id: controller

  required property var host

  readonly property int slotHandleSize: host.slotHandleSize
  readonly property int edgeOpenZone: host.edgeOpenZone
  readonly property var registry: host.registry

  function normalizeEdge(value) { return host.normalizeEdge(value) }
  function slots(edge) { return host.slots(edge) }
  function slotAt(edge, index) { return host.slotAt(edge, index) }
  function slotTops(edge, stackHeight) { return host.slotTops(edge, stackHeight) }
  function slotTitle(edge, index) { return host.slotTitle(edge, index) }
  function slotTabs(edge, index) { return host.slotTabs(edge, index) }
  function slotActiveTab(edge, index) { return host.slotActiveTab(edge, index) }
  function slotModuleAt(edge, index, tabIndex) { return host.slotModuleAt(edge, index, tabIndex) }
  function normalizedFractions(edge) { return host.normalizedFractions(edge) }
  function isOpen(edge) { return host.isOpen(edge) }
  function isWindowMode(edge) { return host.isWindowMode(edge) }
  function isDockedOpen(edge) { return host.isDockedOpen(edge) }
  function bladeWidth(edge) { return host.bladeWidth(edge) }
  function setOpen(edge, value, persist) { return host.setOpen(edge, value, persist) }
  function focusBlade(edge, targetScreen, slotIndex, part, openIfClosed) { return host.focusBlade(edge, targetScreen, slotIndex, part, openIfClosed) }
  function moveSlotTo(sourceEdge, sourceIndex, targetEdge, targetIndex, tabIndex) { return host.moveSlotTo(sourceEdge, sourceIndex, targetEdge, targetIndex, tabIndex) }
  function moveTabInto(sourceEdge, sourceIndex, tabIndex, targetEdge, targetSlot, insertAt) { return host.moveTabInto(sourceEdge, sourceIndex, tabIndex, targetEdge, targetSlot, insertAt) }

  property bool dragActive: false
  property string dragEdge: ""
  property int dragIndex: -1
  property string dragSlotId: ""
  property string dragTitle: ""
  property string dragGlyph: ""
  property string dragScope: "screen"
  property var dragScreen: null
  property real dragScreenX: 0
  property real dragScreenY: 0
  property real dragLocalY: 0
  property real dragStackHeight: 0
  property string dropEdge: ""
  property int dropIndex: -1
  property int dragTab: -1
  property int dropTabSlot: -1
  property string dropTabBand: ""
  property int dropTabIndex: -1
  property bool dropNoop: true
  property string dropLabel: ""
  property string dragAutoOpened: ""

  function beginSlotDrag(edge, slotIndex, targetScreen, scope, screenX, screenY, localY, stackHeight, tabIndex) {
    var source = normalizeEdge(edge)
    var slot = slotAt(source, Number(slotIndex))
    if (!slot) return false
    var wanted = Number(tabIndex)
    dragTab = isFinite(wanted) && wanted >= 0 ? Math.min(wanted, slotTabs(source, Number(slotIndex)).length - 1) : -1
    var displayTab = dragTab >= 0 ? dragTab : slotActiveTab(source, Number(slotIndex))
    var moduleId = slotModuleAt(source, Number(slotIndex), displayTab)
    var module = registry.module(moduleId)
    dragEdge = source
    dragIndex = Number(slotIndex)
    dragSlotId = String(slot.id || "")
    dragTitle = module ? String(module.name) : moduleId
    dragGlyph = module && module.glyph ? String(module.glyph) : "󰏗"
    dragScope = String(scope || "screen") === "local" ? "local" : "screen"
    dragScreen = targetScreen || null
    dragStackHeight = Math.max(1, Number(stackHeight) || 1)
    dragAutoOpened = ""
    dragActive = true
    updateSlotDrag(screenX, screenY, localY)
    return true
  }

  function slotHeightAt(edge, index, stackHeight) {
    var count = slots(edge).length
    var tops = slotTops(edge, stackHeight)
    var usable = Math.max(0, stackHeight - Math.max(0, count - 1) * slotHandleSize)
    return index === count - 1 ? Math.max(0, stackHeight - tops[index]) : usable * normalizedFractions(edge)[index]
  }

  function dropIndexAt(edge, localY, stackHeight) {
    var count = slots(edge).length
    if (count === 0) return 0
    var tops = slotTops(edge, stackHeight)
    for (var i = 0; i < count; i++)
      if (localY < tops[i] + slotHeightAt(edge, i, stackHeight) / 2) return i
    return count
  }

  function describeDrop(edge, index, noop) {
    if (noop) return "Already here"
    var list = slots(edge)
    var side = edge === "right" ? "right blade" : "left blade"
    if (list.length === 0) return "Into the " + side
    var before = -1
    var after = -1
    var sourceSlotMoves = wholeSlotDrag()
    for (var i = 0; i < list.length; i++) {
      if (sourceSlotMoves && edge === dragEdge && i === dragIndex) continue
      if (i < index) before = i
      else if (after < 0) after = i
    }
    var suffix = edge === dragEdge ? "" : " in the " + side
    if (after >= 0) return "Above " + slotTitle(edge, after) + suffix
    if (before >= 0) return "Below " + slotTitle(edge, before) + suffix
    return "Into the " + side
  }

  function describeTabDrop(edge, tabSlot, tabIndex) {
    var list = slotTabs(edge, tabSlot)
    if (dropTabBand !== "tabs" || tabIndex < 0 || list.length === 0) return "Tab with " + slotTitle(edge, tabSlot)
    if (tabIndex < list.length) return "Before " + host.moduleTitle(list[tabIndex].module)
    return "After " + host.moduleTitle(list[list.length - 1].module)
  }

  function dockedDropTarget(screenWidth) {
    if (isDockedOpen("left") && dragScreenX <= bladeWidth("left")) return "left"
    if (isDockedOpen("right") && screenWidth > 0 && dragScreenX >= screenWidth - bladeWidth("right")) return "right"
    return ""
  }

  function canAutoOpenDrop(edge, screenWidth) {
    if (edge === "right")
      return dragScreenX >= screenWidth - edgeOpenZone && !isOpen(edge) && !isWindowMode(edge) && dragEdge !== edge
    return dragScreenX <= edgeOpenZone && !isOpen(edge) && !isWindowMode(edge) && dragEdge !== edge
  }

  function autoOpenDropTarget(screenWidth) {
    if (screenWidth <= 0) return ""
    if (canAutoOpenDrop("right", screenWidth)) {
      setOpen("right", true, true)
      dragAutoOpened = "right"
      return "right"
    }
    if (canAutoOpenDrop("left", screenWidth)) {
      setOpen("left", true, true)
      dragAutoOpened = "left"
      return "left"
    }
    return ""
  }

  function dragTarget(screenWidth) {
    if (dragScope === "local") return dragEdge
    return dockedDropTarget(screenWidth) || autoOpenDropTarget(screenWidth)
  }

  function clearDropTarget() {
    dropEdge = ""
    dropIndex = -1
    dropTabSlot = -1
    dropTabBand = ""
    dropTabIndex = -1
    dropNoop = true
    dropLabel = "Release to cancel"
  }

  function wholeSlotDrag() {
    return dragTab < 0 || slotTabs(dragEdge, dragIndex).length === 1
  }

  function dropIsNoop(target, index, tabSlot, tabIndex) {
    if (tabSlot < 0) return wholeSlotDrag() && target === dragEdge && (index === dragIndex || index === dragIndex + 1)
    if (target !== dragEdge || tabSlot !== dragIndex) return false
    if (dropTabBand !== "tabs" || tabIndex < 0 || dragTab < 0) return true
    return tabIndex === dragTab || tabIndex === dragTab + 1
  }

  function tabDropSlotAt(edge, localY, stackHeight) {
    var count = slots(edge).length
    var tops = slotTops(edge, stackHeight)
    var tabDrag = !wholeSlotDrag()
    for (var i = 0; i < count; i++) {
      var height = slotHeightAt(edge, i, stackHeight)
      var band = tabDrag ? Math.min(host.tabEdgeZone, height / 2) : height * 0.34
      if (localY >= tops[i] + band && localY < tops[i] + height - band) return i
    }
    return -1
  }

  function tabBarSlotAt(edge, localY, stackHeight) {
    var count = slots(edge).length
    var tops = slotTops(edge, stackHeight)
    for (var i = 0; i < count; i++) {
      var height = slotHeightAt(edge, i, stackHeight)
      if (height <= host.tabBarHeight * 3 || host.slotCollapsed(edge, i)) continue
      var start = tops[i] + (i === 0 ? host.tabEdgeZone : 0)
      if (localY >= start && localY < tops[i] + host.tabBarHeight * 1.5) return i
    }
    return -1
  }

  function refreshDropState() {
    dropNoop = dropIsNoop(dropEdge, dropIndex, dropTabSlot, dropTabIndex)
    dropLabel = dropNoop
      ? "Already here"
      : (dropTabSlot >= 0 ? describeTabDrop(dropEdge, dropTabSlot, dropTabIndex) : describeDrop(dropEdge, dropIndex, false))
  }

  function setDropTabIndex(index) {
    if (!dragActive || dropTabBand !== "tabs") return
    var wanted = Number(index)
    dropTabIndex = isFinite(wanted) && wanted >= 0 ? Math.floor(wanted) : -1
    refreshDropState()
  }

  function resolveTabBand(target) {
    var barSlot = tabBarSlotAt(target, dragLocalY, dragStackHeight)
    if (barSlot >= 0) {
      if (dropTabSlot !== barSlot || dropTabBand !== "tabs") dropTabIndex = -1
      dropTabSlot = barSlot
      dropTabBand = "tabs"
      return
    }
    dropTabSlot = tabDropSlotAt(target, dragLocalY, dragStackHeight)
    dropTabBand = dropTabSlot >= 0 ? "body" : ""
    dropTabIndex = -1
  }

  function updateSlotDrag(screenX, screenY, localY) {
    if (!dragActive) return
    dragScreenX = Number(screenX) || 0
    dragScreenY = Number(screenY) || 0
    dragLocalY = Number(localY) || 0
    var screenWidth = dragScreen ? Number(dragScreen.width) || 0 : 0
    var target = dragTarget(screenWidth)
    if (target === "") {
      clearDropTarget()
      return
    }
    if (target !== dropEdge) dropTabIndex = -1
    dropEdge = target
    resolveTabBand(target)
    dropIndex = dropIndexAt(target, dragLocalY, dragStackHeight)
    refreshDropState()
  }

  function currentDrop() {
    var tabTarget = dropTabSlot >= 0 ? slotAt(dropEdge, dropTabSlot) : null
    return {
      target: dropEdge,
      index: dropIndex,
      source: dragEdge,
      sourceIndex: dragIndex,
      sourceTab: dragTab,
      tabSlot: dropTabSlot,
      tabIndex: dropTabBand === "tabs" ? dropTabIndex : -1,
      tabTargetId: tabTarget ? String(tabTarget.id || "") : "",
      noop: dropNoop,
      autoOpened: dragAutoOpened
    }
  }

  function resetDrag() {
    dragActive = false
    clearDropTarget()
    dropLabel = ""
    dragAutoOpened = ""
  }

  function performDrop(drop, commit) {
    if (!commit || drop.target === "" || drop.noop) return false
    return drop.tabSlot >= 0
      ? moveTabInto(drop.source, drop.sourceIndex, drop.sourceTab, drop.target, drop.tabSlot, drop.tabIndex)
      : moveSlotTo(drop.source, drop.sourceIndex, drop.target, drop.index, drop.sourceTab)
  }

  function focusMovedDrop(drop) {
    var destination = drop.tabTargetId
      ? host.findSlotId(drop.target, drop.tabTargetId)
      : Math.min(drop.index, slots(drop.target).length - 1)
    if (!drop.tabTargetId && drop.source === drop.target && drop.index > drop.sourceIndex)
      destination = Math.max(0, destination - 1)
    Qt.callLater(function() { focusBlade(drop.target, dragScreen, destination, "", false) })
  }

  function endSlotDrag(commit) {
    if (!dragActive) return false
    var drop = currentDrop()
    resetDrag()
    var moved = performDrop(drop, commit)
    if (drop.autoOpened !== "" && !(moved && drop.target === drop.autoOpened)) setOpen(drop.autoOpened, false, true)
    if (moved) focusMovedDrop(drop)
    return moved
  }
}
