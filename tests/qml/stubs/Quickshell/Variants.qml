import QtQuick
Item { id: v; property alias model: rep.model; default property alias delegate: rep.delegate; Repeater { id: rep } }
