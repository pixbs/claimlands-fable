(()=>{
  const root=document.getElementById('capital-composer'),el=id=>root.querySelector('#cc-'+id);
  if(typeof THREE==='undefined'){el('error').hidden=false;el('error').textContent='The 3D library could not load. Reopen this preview to retry.';return;}
  const T=THREE,WP=1,TILE_PX=24,R=TILE_PX/2*.94,APOTHEM=R*Math.sqrt(3)/2,INSET=.5,EPS=.015;
  const familyNames={A:'Castle A · single keep',B:'Castle B · twin gate',C:'Castle C · courtyard'};
  const {generate,validate,inside,hash}=CapitalLayout;
  function rng(s){let a=hash(s);return()=>{a=(a+0x6d2b79f5)>>>0;let t=Math.imul(a^(a>>>15),1|a);t^=t+Math.imul(t^(t>>>7),61|t);return ((t^(t>>>14))>>>0)/4294967296;};}
  root.castleEngine={generate,validate,inside};
  const canvas=el('main');let renderer;
  try{renderer=new T.WebGLRenderer({alpha:true,antialias:false,preserveDrawingBuffer:true});}catch(e){el('error').hidden=false;el('error').textContent='WebGL is required to view these castles.';return;}
  renderer.setPixelRatio(1);renderer.outputEncoding=T.LinearEncoding;renderer.setClearColor(0,0);
  const scene=new T.Scene(),camera=new T.OrthographicCamera(-20,20,20,-20,.1,200);scene.add(new T.AmbientLight('#b9c6ff',.66));const sun=new T.DirectionalLight('#fff2d8',.52);sun.position.set(1.1,.9,1.4);scene.add(sun);
  let group=new T.Group(),fort=new T.Group(),plan=null,currentSelection=null,P,yaw=35*Math.PI/180,pitch=.64,zoom=1,drag=null,spinning=false,raf=0,last=0,inView=true;
  const materials=new Map(),history=[],reduced=matchMedia('(prefers-reduced-motion: reduce)');scene.add(group);
  function material(c){if(!materials.has(c))materials.set(c,new T.MeshLambertMaterial({color:c,side:T.DoubleSide,flatShading:true}));return materials.get(c);}
  function mesh(geo,c,parent=fort){const m=new T.Mesh(geo,material(c));parent.add(m);return m;}
  function box(x,y,z,w,h,d,c,parent=fort){if(w<=0||h<=0||d<=0)return;const m=mesh(new T.BoxGeometry(w,h,d),c,parent);m.position.set(x,y+h/2,z);return m;}
  function poly(vertices,c,parent=fort){const pos=[];for(let i=1;i<vertices.length-1;i++)pos.push(...vertices[0],...vertices[i],...vertices[i+1]);const g=new T.BufferGeometry();g.setAttribute('position',new T.Float32BufferAttribute(pos,3));g.computeVertexNormals();return mesh(g,c,parent);}
  function ground(){
    const random=rng(plan.seed+'|ground'),tc=document.createElement('canvas');tc.width=tc.height=TILE_PX;const c=tc.getContext('2d');
    const ca=Math.cos(-plan.angle),sa=Math.sin(-plan.angle);
    for(let z=0;z<24;z++)for(let x=0;x<24;x++){
      const wx=x+.5-12,wz=z+.5-12,lx=wx*ca+wz*sa,lz=-wx*sa+wz*ca;
      const bare=Math.hypot(wx,wz)<6.9+Math.sin(wx*.8)*.6,road=plan.path.some(p=>Math.abs(lx-p.x)<=p.w/2&&Math.abs(lz-p.z)<=p.d/2);
      const value=Math.sin(x*.35+Math.sin(z*.43))+Math.cos(z*.4-x*.17)+(random()-.5)*.3,colors=road?P.path:bare?P.mud:P.grass;
      c.fillStyle=colors[value>.8?2:value<-.8?0:1];c.fillRect(x,z,1,1);
    }
    const tx=new T.CanvasTexture(tc);tx.minFilter=tx.magFilter=T.NearestFilter;tx.generateMipmaps=false;
    const corners=Array.from({length:6},(_,i)=>[R*Math.cos(i*Math.PI/3),R*Math.sin(i*Math.PI/3)]),pos=[],uv=[];
    for(let i=0;i<6;i++)for(const v of [[0,0],corners[(i+1)%6],corners[i]]){pos.push(v[0],0,v[1]);uv.push(.5+v[0]/24,.5+v[1]/24);}
    const geometry=new T.BufferGeometry();geometry.setAttribute('position',new T.Float32BufferAttribute(pos,3));geometry.setAttribute('uv',new T.Float32BufferAttribute(uv,2));geometry.computeVertexNormals();group.add(new T.Mesh(geometry,new T.MeshLambertMaterial({map:tx,side:T.DoubleSide})));
    for(let i=0;i<6;i++){const a=corners[i],b=corners[(i+1)%6],len=R;for(let row=0;row<4;row++)for(let j=0;j<len;j++){
      const f=j/len,q=Math.min(j+1,len)/len,A=[a[0]+(b[0]-a[0])*f,a[1]+(b[1]-a[1])*f],B=[a[0]+(b[0]-a[0])*q,a[1]+(b[1]-a[1])*q];
      poly([[A[0],-row,A[1]],[B[0],-row,B[1]],[B[0],-row-1,B[1]],[A[0],-row-1,A[1]]],P.cliff[random()<.12?Math.max(0,row-1):row],group);
    }}
  }
  function battlements(b){
    const ex=(b.w-1)/2,ez=(b.d-1)/2;
    for(let x=-ex;x<=ex+.001;x+=2)for(const s of [-1,1])box(b.x+x,b.h,b.z+s*ez,1,1,1,P.wall);
    for(let z=-ez+2;z<ez-.001;z+=2)for(const s of [-1,1])box(b.x+s*ex,b.h,b.z+z,1,1,1,P.wall);
  }
  function occupied(x,z,y,except){return [...plan.buildings,...plan.walls].some(b=>b!==except&&Math.abs(x-b.x)<b.w/2+.01&&Math.abs(z-b.z)<b.d/2+.01&&b.h+1>y);}
  function buildTower(b){
    box(b.x,0,b.z,b.w,b.h,b.d,P.wall);box(b.x,b.h+EPS,b.z,b.w-2,EPS,b.d-2,P.floor);battlements(b);
    const wy=Math.max(2,b.h-3);
    for(const s of [-1,1]){
      if(!occupied(b.x+s*(b.w/2+.2),b.z,wy,b))box(b.x+s*(b.w/2+EPS),wy,b.z,EPS,1,1,P.dark);
      if(!occupied(b.x,b.z+s*(b.d/2+.2),wy,b))box(b.x,wy,b.z+s*(b.d/2+EPS),1,1,EPS,P.dark);
    }
  }
  function buildWall(b){
    box(b.x,0,b.z,b.w,b.h,b.d,P.wall);
    const horizontal=b.w>b.d,length=horizontal?b.w:b.d,count=Math.max(1,Math.floor((length+1)/2));
    for(let i=0;i<count;i++){const t=(i-(count-1)/2)*2;box(b.x+(horizontal?t:0),b.h,b.z+(horizontal?0:t),1,1,1,P.wall);}
  }
  function buildFlag(f){
    box(f.x,f.y,f.z,1,f.poleHeight,1,P.wood);
    const shape=new T.Shape(),points=[[0,0],[4,0],[3,-1.5],[4,-3],[0,-3]];points.forEach((p,i)=>i?shape.lineTo(p[0]*f.direction,p[1]):shape.moveTo(p[0]*f.direction,p[1]));shape.closePath();
    const cloth=mesh(new T.ShapeGeometry(shape),el('banner').value);cloth.position.set(f.x+f.direction*.5,f.y+f.poleHeight,f.z);
  }
  function rebuild(skipHistory=false){
    const value=el('seed').value.trim();if(!value){el('error').hidden=false;el('error').textContent='Enter a seed to generate a castle.';return;}
    const prior=plan,selection=el('family').value;
    try{plan=generate(value,el('family').value);}catch(e){el('error').hidden=false;el('error').textContent=e.message;return;}el('error').hidden=true;
    if(!prior||prior.seed!==value||currentSelection!==selection){
      if(prior&&skipHistory!==true)history.push({seed:prior.seed,selection:currentSelection});
      yaw=plan.angle+35*Math.PI/180;sync();
    }
    currentSelection=selection;
    scene.remove(group);group.traverse(o=>{if(o.geometry)o.geometry.dispose();if(o.material&&o.material.map){o.material.map.dispose();o.material.dispose();}});group=new T.Group();fort=new T.Group();group.add(fort);scene.add(group);fort.rotation.y=plan.angle;
    P={wall:'#e7ddc8',floor:'#a99b7e',dark:'#453227',wood:'#665034',grass:['#3f7d34','#5aa444','#7cc255'],mud:['#a18a5b','#c9ac72','#ddbd7d'],path:['#ad956a','#b8a277','#c4b08a'],cliff:['#5a4a2e','#7d6a45','#a08a5f','#bda677']};
    ground();plan.buildings.forEach(buildTower);plan.walls.forEach(buildWall);
    if(plan.gate){const g=plan.gate;box(g.x,2,g.z,g.gap,Math.max(1,g.h-2),1,P.wall);for(const dx of [-1,1])box(g.x+dx,Math.max(3,g.h),g.z,1,1,1,P.wall);}
    box(plan.keep.x,EPS,plan.keep.z+plan.keep.d/2+EPS,2,2,EPS,P.dark);buildFlag(plan.flag);
    const towers=plan.buildings.filter(b=>b.role.includes('tower')).length,wings=plan.buildings.filter(b=>b.role==='wing').length;
    el('detail').textContent=familyNames[plan.family]+' · '+(towers?towers+' towers · ':'')+plan.keep.w+' × '+plan.keep.d+' keep'+(wings?' + side wing':'');
    canvas.setAttribute('aria-label',familyNames[plan.family]+', seed '+plan.seed+'. Drag to rotate.');
    root.dataset.seed=plan.seed;root.dataset.family=plan.family;root.dataset.signature=JSON.stringify(plan);root.dataset.worldPixel=1;root.dataset.tileTexture=24;root.dataset.errors=JSON.stringify(validate(plan));
    root.currentPlan=plan;el('prev').disabled=history.length===0;draw();
  }
  function render(out,halfW,halfH,targetY,width,height,viewPitch=pitch){
    renderer.setSize(width,height,false);camera.left=-halfW;camera.right=halfW;camera.top=halfH;camera.bottom=-halfH;camera.updateProjectionMatrix();
    camera.position.set(Math.sin(yaw)*70*Math.cos(viewPitch),targetY+Math.sin(viewPitch)*70,Math.cos(yaw)*70*Math.cos(viewPitch));camera.lookAt(0,targetY,0);renderer.render(scene,camera);
    if(out.width!==width)out.width=width;if(out.height!==height)out.height=height;const c=out.getContext('2d');c.imageSmoothingEnabled=false;c.clearRect(0,0,width,height);c.drawImage(renderer.domElement,0,0);
  }
  function draw(){if(!root.isConnected||!plan)return;const w=canvas.clientWidth,h=canvas.clientHeight;if(!w||!h)return;const ratio=w/h,halfH=Math.max(14,13/ratio)/zoom;render(canvas,halfH*ratio,halfH,5,Math.round(w/2),Math.round(h/2));for(const [id,d] of [['far',1],['mid',2],['near',3]])render(el(id),48/d,50/d,5,96,100,.64);}
  function sync(){const v=Math.round((yaw*180/Math.PI%360+360)%360);el('orbit').value=v;el('degrees').textContent=v+'°';}
  el('next').onclick=()=>{el('seed').value=String(hash(el('seed').value+'|next'));rebuild();};
  el('prev').onclick=()=>{if(history.length){const previous=history.pop();el('seed').value=previous.seed;el('family').value=previous.selection;rebuild(true);}};
  for(const id of ['seed','family','banner'])el(id).onchange=()=>rebuild();
  el('seed').onkeydown=e=>{if(e.key==='Enter')rebuild();};
  el('view').onchange=()=>{pitch=el('view').value==='plan'?1.48:el('view').value==='front'?.22:.64;draw();};
  el('orbit').oninput=()=>{yaw=+el('orbit').value*Math.PI/180;sync();draw();};
  function active(){return spinning&&inView&&!document.hidden&&root.isConnected;}
  function animate(t){raf=0;if(!active())return;const dt=Math.min((t-last)/1000,.1);last=t;yaw+=dt*.3;sync();draw();raf=requestAnimationFrame(animate);}
  function ensure(){if(active()&&!raf){last=performance.now();raf=requestAnimationFrame(animate);}}
  el('spin').onclick=()=>{spinning=!spinning;el('spin').textContent=spinning?'Pause turntable':'Turntable';el('spin').setAttribute('aria-pressed',String(spinning));ensure();};
  reduced.addEventListener('change',()=>{if(reduced.matches&&spinning)el('spin').click();});
  document.addEventListener('visibilitychange',ensure);new IntersectionObserver(es=>{inView=es[0].isIntersecting;ensure();}).observe(root);
  canvas.addEventListener('pointerdown',e=>{drag={x:e.clientX,y:e.clientY};canvas.setPointerCapture(e.pointerId);});
  canvas.addEventListener('pointermove',e=>{if(!drag)return;yaw-=(e.clientX-drag.x)*.012;pitch=Math.max(.1,Math.min(1.48,pitch+(e.clientY-drag.y)*.008));drag={x:e.clientX,y:e.clientY};sync();draw();});
  for(const event of ['pointerup','pointercancel','lostpointercapture'])canvas.addEventListener(event,()=>drag=null);
  canvas.addEventListener('wheel',e=>{e.preventDefault();zoom=Math.max(.75,Math.min(1.35,zoom*Math.exp(-e.deltaY*.001)));draw();},{passive:false});
  new ResizeObserver(draw).observe(canvas);rebuild();
})();
