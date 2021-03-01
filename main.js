 let canvas;
 window.onload = () => {
     canvas = this.__canvas = new fabric.Canvas('c');
     canvas.backgroundColor = "#424549"
     canvas.on({
        'object:modified': updateControls,
        'object:moving': updateControls,
        'object:scaling': updateControls,
        'object:resizing': updateControls,
        'object:rotating': updateControls,
        'object:skewing': updateControls
     });
 }

 function addRect() {
     canvas.add(new fabric.Rect({ left: 110, top: 110, fill: '#3bb3d1',opacity:0.1, width: 50, height: 50 }));
 }
 function addImage(data) {
     fabric.Image.fromURL(URL.createObjectURL(data.target.files[0]), img => {
        img.x = 0,
        img.y = 0
        canvas.add(img).setActiveObject(img);
     });
 }

 function updateControls(element) {
    document.getElementById("setx").value = element.target.left
    document.getElementById("sety").value = element.target.top
    document.getElementById("data").innerHTML = `width: ${element.target.width*element.target.scaleX} height: ${element.target.height*element.target.scaleY}`
    canvas.renderAll();
 }

 function updateX() {
     canvas._activeObject.left = Number(document.getElementById("setx").value)
     canvas.renderAll();

 }
 function updateY() {
     canvas._activeObject.top = Number(document.getElementById("sety").value)
     canvas.renderAll();

 }