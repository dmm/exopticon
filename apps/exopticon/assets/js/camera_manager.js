


Array.prototype.diff = function(a) {
    return this.filter(function(i) {
        return a.indexOf(i) < 0;
    });
};

function isVisible(element) {
    let rec = element.getBoundingClientRect();
    let viewportHeight = window.innerHeight;
    let y = window.scrollY;
    let bottomEdge = y + viewportHeight;

    if ((rec.bottom > 0 && rec.bottom < viewportHeight)
        || (rec.top > 0 && rec.top < viewportHeight)) {
        return true;
    }

    return false;
}

function renderFrame(img, imageArrayBuffer) {
    var blob  = new Blob([imageArrayBuffer],{type: "image/jpeg"});
    img.onload = function (e) {
        window.URL.revokeObjectURL(img.src);
        img = null;
    };

    img.onerror = img.onabort = function () {
        console.log('error loading image!');
        img = null;
    };
    img.src = window.URL.createObjectURL(blob);
}

var camera = function(id, name) {
    this.id = id;
    this.name = name;

    return this;
};

function ptzClickHandler(event) {
    const id = event.target.getAttribute('data-camera-id');
    let x = 0.0;
    let y = 0.0;
    const cn = event.target.className;
    if (cn.indexOf("ptz-left") != -1) {
        x = '-0.05';
    } else if (cn.indexOf("ptz-up") != -1) {
        y = '0.1';
    } else if (cn.indexOf("ptz-right") != -1) {
        x = '0.05';
    } else if (cn.indexOf("ptz-down") != -1) {
        y = '-0.1';
    }
    var request = new XMLHttpRequest();
    request.open("POST", `/v1/cameras/${id}/relativeMove`);
    request.setRequestHeader("Content-type", "application/json");
    request.send(JSON.stringify({x: x, y: y}));

}

function getPtzElement(id) {
    var div = document.createElement('div');
    div.innerHTML  = `<div class="ptz-controls">
                        <div class="ptz-left" data-camera-id="${id}"></div>
                        <div class="ptz-up" data-camera-id="${id}"></div>
                        <div class="ptz-right" data-camera-id="${id}"></div>
                        <div class="ptz-down" data-camera-id="${id}"></div>
                     </div>`;

    div.childNodes[0].childNodes.forEach(function(element) {
        element.onclick = ptzClickHandler;
    });
    return div.childNodes[0];
}

var CameraManager = function(socket) {
    this.cameras = new Map();
    this.channels = new Map();
    this.socket = socket;
    this.visibleCameras = new Map();
    this.checkingVisibility = false;
    this.lastScrollPosition = 0;
    let self = this;
    window.addEventListener('scroll', (e) => {
        this.lastScrollPosition = window.scrollY;

        if (!this.checkingVisibility) {
            this.checkingVisibility = true;
            window.requestAnimationFrame(() => {
                self.visibleCameras.clear();
                self.cameras.forEach(function (value, key) {
                    let element = document.querySelector('#camera' + key);
                    if (isVisible(element)) {
                        console.log(key + ' is visible.');
                        self.visibleCameras.set(key, true);
                    } else {
                        console.log(key + ' is not visible.');
                    }
                });
                this.checkingVisibility = false;
            });
        }
    });
};

CameraManager.prototype = {
    startNewCamera: function(newCamera) {
        let channel = this.socket.channel("camera:" + newCamera.id);
        newCamera.channel = channel;
        let imgDiv = document.createElement('div');
        imgDiv.id = 'camera' + newCamera.id;
        imgDiv.className = 'camera';
        if (newCamera.ptzType !== null) {
            imgDiv.className += ' ptz';
            imgDiv.appendChild(getPtzElement(newCamera.id));
        }
        let img = document.createElement("img");
        imgDiv.appendChild(img);

        let videoContainer = document.getElementById("allCameras");
        videoContainer.appendChild(imgDiv);
        channel.on("jpg", (data) => {
            if (this.visibleCameras.has(newCamera.id)) {
                renderFrame(img, data.frameJpeg);
            }

            channel.push("ack", "");
        });
        this.visibleCameras.set(newCamera.id, true);
        channel.join();
        this.cameras.set(newCamera.id, newCamera);
    },

    removeCamera: function(oldCamera) {
        var channel = this.oldCamera.channel;
        channel.leave();
        this.cameras = _.filter(this.cameras, function(c) {
            return c.id != oldCamera.id;
        });
        var element = document.getElementById('camera'+oldCamera.id);
        element.outerHTML = '';
        this.cameras.delete(oldCamera.id);
    },

    updateCameras: function(allCameras) {
        let curMap = new Map();
        let self = this;

        allCameras.forEach(function(c) {
            curMap.set(c.id, c);
            if (!self.cameras.has(c.id)) {
                self.startNewCamera(c);
            }
        });

        for (var [key, value] of this.cameras) {
            if (!curMap.has(key)) {
                // Camera has been removed
                self.removeCamera(value);
            }
        }
    }
};

export default CameraManager;
