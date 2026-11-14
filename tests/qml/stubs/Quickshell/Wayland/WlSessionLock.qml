import QtQuick
Item { id: l; property bool locked: false; default property Component surface; Loader { active: l.locked; sourceComponent: l.surface } }
