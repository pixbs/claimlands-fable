(() => {
  const root=document.getElementById('wp-refined-study'),el=id=>root.querySelector('#wp-'+id);
  if(typeof THREE==='undefined'){el('error').hidden=false;el('error').textContent='The 3D library could not load. Reopen the preview to retry.';return;}
  const T=THREE;
  // Prototype dimensions in world pixels. A study unit is exactly one WP.
  const canvas=el('canvas');let renderer;
  try{renderer=new T.WebGLRenderer({antialias:false,alpha:true,preserveDrawingBuffer:true});}
  catch(e){el('error').hidden=false;el('error').textContent='WebGL is required for this preview.';return;}
  renderer.setPixelRatio(1);renderer.outputEncoding=T.LinearEncoding;renderer.setClearColor(0,0);
  const scene=new T.Scene(),camera=new T.OrthographicCamera(-20,20,22,-22,.1,200);
  scene.add(new T.AmbientLight('#b9c6ff',.66));const sun=new T.DirectionalLight('#fff2d8',.52);sun.position.set(1.1,.9,1.4);scene.add(sun);
  let world=new T.Group(),scenery=new T.Group(),token=null,star=null;scene.add(world);
  let yaw=35*Math.PI/180,pitch=.24,zoom=1,spinning=false,drag=null,raf=0,last=0,starTime=0,lastDraw=0,inView=true;
  const STAR_RADIUS=AvailabilityStar.radius,STAR_PERIOD=AvailabilityStar.period;
  let unitBottom=0,unitTop=0,coverTop=0;
  const reduced=matchMedia('(prefers-reduced-motion: reduce)');
  const standalone=root.dataset.design==='star';
  function rebuild(){
    world.traverse(o=>{if(o.geometry)o.geometry.dispose();if(o.material){if(o.material.map)o.material.map.dispose();o.material.dispose();}});scene.remove(world);
    world=new T.Group();scenery=new T.Group();world.add(scenery);scene.add(world);
    const kit=ReferenceKit(scenery,el('faction').value),cover=el('cover').value;
    kit.ground(cover);if(cover==='village')kit.village();
    if(cover==='forest')for(const args of [[-4,-3,1.15],[2,-4,1.3],[5,2,.92],[-4,3,1],[0,3,1.4]])kit.tree(...args);
    coverTop=Math.max(0,new T.Box3().setFromObject(scenery).max.y);
    unitBottom=Math.ceil(coverTop)+2;unitTop=coverTop;token=null;
    if(!standalone){token=ReferenceModel(kit);token.position.y=unitBottom;world.add(token);unitTop=new T.Box3().setFromObject(token).max.y;}
    star=AvailabilityStar.build(kit);star.position.y=Math.ceil(unitTop)+2+STAR_RADIUS;world.add(star);star.visible=el('star').checked;
    root.dataset.unitHeight=token?unitTop-unitBottom:0;root.dataset.unitBottom=unitBottom;root.dataset.coverTop=coverTop;
    root.dataset.worldPixel=1;root.dataset.tileTexture=24;root.dataset.starPeriod=STAR_PERIOD;
    root.inspect=()=>({token,star,camera,world});
    el('detail').textContent=standalone?'Five clipped points · 6 WP across · one Y revolution every 8 seconds':(unitTop-unitBottom).toFixed(1)+' WP tall · 2 WP clearance above scenery';
    draw();ensureAnimation();
  }
  function renderTo(out,density,halfW,halfH,targetY,width,height){
    renderer.setSize(width,height,false);camera.left=-halfW;camera.right=halfW;camera.top=halfH;camera.bottom=-halfH;camera.updateProjectionMatrix();
    camera.position.set(Math.sin(yaw)*70*Math.cos(pitch),targetY+Math.sin(pitch)*70,Math.cos(yaw)*70*Math.cos(pitch));camera.lookAt(0,targetY,0);
    AvailabilityStar.orient(star,camera,starTime);
    root.dataset.starAngle=starTime*Math.PI*2/STAR_PERIOD;
    root.dataset.starFacing=new T.Vector3(0,0,1).applyQuaternion(star.quaternion).dot(new T.Vector3(0,0,1).applyQuaternion(camera.quaternion));
    root.dataset.starUp=new T.Vector3(0,1,0).applyQuaternion(star.quaternion).dot(new T.Vector3(0,1,0).applyQuaternion(camera.quaternion));
    renderer.render(scene,camera);
    if(out.width!==width)out.width=width;if(out.height!==height)out.height=height;const c=out.getContext('2d');c.imageSmoothingEnabled=false;c.clearRect(0,0,width,height);c.drawImage(renderer.domElement,0,0);
  }
  function draw(){
    if(!root.isConnected)return;const cw=canvas.clientWidth,ch=canvas.clientHeight,ratio=cw/ch;
    const comparisonTop=token?unitTop:coverTop;
    const maxY=star.visible?Math.ceil(comparisonTop)+2+STAR_RADIUS*2:comparisonTop;
    const close=el('framing').value==='figure',lowY=close?(standalone?star.position.y-STAR_RADIUS:unitBottom):-4,targetY=(maxY+lowY)/2;
    const halfH=Math.max(close?11:18,(maxY-lowY)*.60,(close?8:16)/ratio)/zoom,halfW=halfH*ratio;
    scenery.visible=!close;
    renderTo(canvas,0,halfW,halfH,targetY,Math.max(1,Math.round(cw/2)),Math.max(1,Math.round(ch/2)));
    root.dataset.frameHeight=halfH*2;
    scenery.visible=true;
    const inspectionPitch=pitch;pitch=.64;
    // Fixed screen-pixel density: these are actual distant views, not magnified thumbnails.
    for(const [id,d] of [[1,1],[2,2],[4,3]])renderTo(el('lod'+id),d,48/d,80/d,(maxY-4)/2,96,160);
    pitch=inspectionPitch;
  }
  function sync(){const deg=Math.round((yaw*180/Math.PI%360+360)%360);el('orbit').value=deg;el('deg').textContent=deg+'°';}
  for(const id of ['cover','faction'])el(id).onchange=rebuild;
  el('framing').onchange=draw;
  el('view').onchange=()=>{pitch=el('view').value==='top'?1.46:el('view').value==='side'?.24:.64;draw();};
  el('orbit').oninput=()=>{yaw=+el('orbit').value*Math.PI/180;sync();draw();};
  el('star').onchange=()=>{star.visible=el('star').checked;draw();ensureAnimation();};
  el('spin').onclick=()=>{spinning=!spinning;el('spin').setAttribute('aria-pressed',String(spinning));el('spin').textContent=spinning?'Pause turntable':'Turntable';ensureAnimation();};
  function active(){return root.isConnected&&inView&&!document.hidden&&(spinning||(star&&star.visible&&!reduced.matches));}
  function ensureAnimation(){if(active()&&!raf){last=performance.now();raf=requestAnimationFrame(tick);}}
  function tick(t){raf=0;if(!active())return;const elapsed=(t-last)/1000,dt=Math.min(elapsed,.1);last=t;if(spinning){yaw+=dt*.3;sync();}if(star.visible&&!reduced.matches)starTime+=elapsed;if(t-lastDraw>=1000/30){draw();lastDraw=t;}raf=requestAnimationFrame(tick);}
  reduced.addEventListener('change',()=>{if(reduced.matches&&spinning)el('spin').click();ensureAnimation();});
  document.addEventListener('visibilitychange',ensureAnimation);
  new IntersectionObserver(entries=>{inView=entries[0].isIntersecting;ensureAnimation();}).observe(root);
  canvas.addEventListener('pointerdown',e=>{drag={x:e.clientX,y:e.clientY};canvas.setPointerCapture(e.pointerId);});
  canvas.addEventListener('pointermove',e=>{if(!drag)return;yaw-=(e.clientX-drag.x)*.012;pitch=Math.max(.08,Math.min(1.48,pitch+(e.clientY-drag.y)*.008));drag={x:e.clientX,y:e.clientY};sync();draw();});
  for(const event of ['pointerup','pointercancel','lostpointercapture'])canvas.addEventListener(event,()=>drag=null);
  canvas.addEventListener('wheel',e=>{e.preventDefault();zoom=Math.max(.75,Math.min(1.4,zoom*Math.exp(-e.deltaY*.001)));draw();},{passive:false});
  new ResizeObserver(draw).observe(canvas);rebuild();
})();
