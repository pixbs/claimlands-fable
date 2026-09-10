// Shared geometry and comparison scenery; box y specifies the bottom.
window.ReferenceKit=function(scenery,faction='#4c92b2'){
  const T=THREE,WP=1,TILE_PX=24,R=12*.94,WINDOW=1,DOOR=2,ROOF_OVER=1,ROOF_LIP=1,ROOF_COURSE=2,CROWN_W=10,CROWN_H=7,DECAL=.02;
  const P={wall:'#e7ddc8',stoneShade:'#a99b7e',roof:'#a24b32',roofDark:'#823a26',wood:'#665034',dark:'#453227',green:'#3a7a26',floor:'#24451b',grass:['#3f7d34','#5aa444','#7cc255'],mud:['#a18a5b','#c9ac72','#ddbd7d'],cliff:['#5a4a2e','#7d6a45','#a08a5f','#bda677'],skin:'#e2c599',metal:'#bac2ba',gold:'#f2cc64',faction};
  let seed=1729;const materials=new Map();
  const corners=Array.from({length:6},(_,i)=>[R*Math.cos(i*Math.PI/3),R*Math.sin(i*Math.PI/3)]);
  function rnd(){seed=(Math.imul(seed,1664525)+1013904223)>>>0;return seed/4294967296;}
  function material(c,unlit=false){const k=c+unlit;if(!materials.has(k))materials.set(k,unlit?new T.MeshBasicMaterial({color:c,side:T.DoubleSide}):new T.MeshLambertMaterial({color:c,flatShading:true,side:T.DoubleSide}));return materials.get(k);}
  function mesh(g,c,parent=scenery,unlit=false){const m=new T.Mesh(g,material(c,unlit));parent.add(m);return m;}
  function box(x,y,z,w,h,d,c,parent=scenery){const m=mesh(new T.BoxGeometry(w,h,d),c,parent);m.position.set(x,y+h/2,z);return m;}
  function poly(v,c,parent=scenery){const arr=[];for(let i=1;i<v.length-1;i++)arr.push(...v[0],...v[i],...v[i+1]);const g=new T.BufferGeometry();g.setAttribute('position',new T.Float32BufferAttribute(arr,3));g.computeVertexNormals();return mesh(g,c,parent);}
  function tapered(x,y,z,w0,d0,w1,d1,h,c,parent=scenery){
    const bottom=[[-w0/2,0,-d0/2],[w0/2,0,-d0/2],[w0/2,0,d0/2],[-w0/2,0,d0/2]].map(v=>[x+v[0],y,z+v[2]]);
    const top=[[-w1/2,h,-d1/2],[w1/2,h,-d1/2],[w1/2,h,d1/2],[-w1/2,h,d1/2]].map(v=>[x+v[0],y+h,z+v[2]]);
    poly(top,c,parent);poly(bottom.slice().reverse(),c,parent);for(let i=0;i<4;i++)poly([bottom[i],bottom[(i+1)%4],top[(i+1)%4],top[i]],c,parent);
  }
  function cutBox(x,y,z,w,h,d,cut,c,parent=scenery){
    const hw=w/2,hd=d/2,outline=[[-hw+cut,-hd],[hw-cut,-hd],[hw,-hd+cut],[hw,hd-cut],[hw-cut,hd],[-hw+cut,hd],[-hw,hd-cut],[-hw,-hd+cut]];
    const lo=outline.map(p=>[x+p[0],y,z+p[1]]),hi=outline.map(p=>[x+p[0],y+h,z+p[1]]);
    poly(hi,c,parent);poly(lo.slice().reverse(),c,parent);for(let i=0;i<8;i++)poly([lo[i],lo[(i+1)%8],hi[(i+1)%8],hi[i]],c,parent);
  }
  function flat(points,c,parent=scenery){const s=new T.Shape();points.forEach((p,i)=>i?s.lineTo(p[0],p[1]):s.moveTo(p[0],p[1]));s.closePath();return mesh(new T.ShapeGeometry(s),c,parent);}
  function tint(c,n){return '#'+new T.Color(c).multiplyScalar(n).getHexString();}
  function ground(cover){
    // 24 texels cover 24 WP, with the prototype's .94 UV inset.
    const texCanvas=document.createElement('canvas');texCanvas.width=TILE_PX;texCanvas.height=TILE_PX;const tc=texCanvas.getContext('2d');
    for(let z=0;z<TILE_PX;z++)for(let x=0;x<TILE_PX;x++){
      const wx=x+.5-12,wz=z+.5-12,wave=Math.sin(x*.35+Math.sin(z*.43))+Math.cos(z*.4-x*.17)+(rnd()-.5)*.5;
      const bare=cover==='village',mud=bare&&Math.hypot(wx,wz)<6.6+Math.sin(x*1.7)*1.8+Math.cos(z*1.1);
      const palette=mud?P.mud:P.grass;tc.fillStyle=palette[wave>.8?2:wave<-.8?0:1];tc.fillRect(x,z,1,1);
    }
    const texture=new T.CanvasTexture(texCanvas);texture.magFilter=T.NearestFilter;texture.minFilter=T.NearestFilter;texture.generateMipmaps=false;
    const pos=[],uv=[];for(let i=0;i<6;i++)for(const v of [[0,0],corners[(i+1)%6],corners[i]]){pos.push(v[0],0,v[1]);uv.push(.5+v[0]/TILE_PX,.5+v[1]/TILE_PX);}
    const g=new T.BufferGeometry();g.setAttribute('position',new T.Float32BufferAttribute(pos,3));g.setAttribute('uv',new T.Float32BufferAttribute(uv,2));g.computeVertexNormals();scenery.add(new T.Mesh(g,new T.MeshLambertMaterial({map:texture,side:T.DoubleSide})));
    // Each cliff row is one WP tall, matching the prototype's four-row strip.
    for(let i=0;i<6;i++){const a=corners[i],b=corners[(i+1)%6],len=Math.hypot(b[0]-a[0],b[1]-a[1]);
      for(let row=0;row<4;row++)for(let start=0;start<len;start+=WP){const end=Math.min(start+WP,len),f=start/len,q=end/len,ax=a[0]+(b[0]-a[0])*f,az=a[1]+(b[1]-a[1])*f,bx=a[0]+(b[0]-a[0])*q,bz=a[1]+(b[1]-a[1])*q;let k=row;if(rnd()<.12)k=Math.max(0,row-1);poly([[ax,-row,az],[bx,-row,bz],[bx,-row-1,bz],[ax,-row-1,az]],P.cliff[k]);}
    }
  }
  function tree(x,z,scale=1){
    const radius=CROWN_W*.5*scale,h=CROWN_H*scale,bottom=-1.2;
    const floor=mesh(new T.CircleGeometry(radius*1.1,7),P.floor);floor.rotation.x=-Math.PI/2;floor.position.set(x,.05,z);
    const profile=[[-.10,.55],[.30,.92],[.60,1],[.85,.60]],shades=[.38,.62,.88,1.18],rings=[];
    for(let j=0;j<4;j++){const ring=[],tw=j*.45+rnd()*.6;for(let i=0;i<7;i++){const a=i*Math.PI*2/7+tw,r=radius*profile[j][1]*(.91+rnd()*.18);ring.push([x+Math.cos(a)*r,bottom+h*profile[j][0],z+Math.sin(a)*r]);}rings.push(ring);}
    for(let j=0;j<3;j++)for(let i=0;i<7;i++){const n=(i+1)%7;poly([rings[j][i],rings[j][n],rings[j+1][n]],tint(P.green,shades[j]*(.975+rnd()*.05)));poly([rings[j][i],rings[j+1][n],rings[j+1][i]],tint(P.green,shades[j]*(.975+rnd()*.05)));}
    const apex=[x+.2*scale,bottom+h,z-.2*scale];for(let i=0;i<7;i++)poly([rings[3][i],rings[3][(i+1)%7],apex],tint(P.green,1.18));
  }
  function village(){
    house(-4.5,-4.5,4.5,6,3.7,0,true);house(4.5,-4.5,4,5,3.6,Math.PI/2,false);house(-4.5,4.5,4,5,3.5,Math.PI/4,false);house(4.5,4.5,4.5,6,3.8,0,true);
  }
  function house(x,z,w=4.5,d=6,h=3.7,rot=0,striped=true){
    const b=new T.Group();scenery.add(b);b.position.set(x,-.6,z);b.rotation.y=rot;
    const hw=w/2,hd=d/2,W=hw+ROOF_OVER,D=hd+ROOF_OVER,eb=h-ROOF_OVER,et=eb+ROOF_LIP*Math.SQRT2,rt=et+W,rb=eb+W,mit=ROOF_LIP/Math.SQRT2;
    box(0,0,0,w,h,d,P.wall,b);for(const e of [-1,1])poly([[-hw,h,e*hd],[hw,h,e*hd],[0,h+hw,e*hd]],P.wall,b);
    for(const s of [-1,1]){
      const xE=s*W,xC=s*Math.min(W*.5,1.5),yC=et+W-Math.abs(xC);
      const steps=Math.max(2,Math.round(2*D/ROOF_COURSE));
      for(let i=0;i<steps;i++){const z0=-D+2*D*i/steps,z1=-D+2*D*(i+1)/steps;poly([[xE,et,z0],[xE,et,z1],[xC,yC,z1],[xC,yC,z0]],striped&&(i&1)?P.roofDark:P.roof,b);}
      poly([[xC,yC,-D],[xC,yC,D],[0,rt,D],[0,rt,-D]],P.roofDark,b);
      poly([[xE,et,-D],[xE,et,D],[s*(W-mit),eb+mit,D],[s*(W-mit),eb+mit,-D]],P.roofDark,b);
      for(const e of [-1,1])poly([[xE,et,e*D],[0,rt,e*D],[0,rb,e*D],[s*(W-mit),eb+mit,e*D]],P.roofDark,b);
      poly([[s*(W-mit),eb+mit,-D],[s*(W-mit),eb+mit,D],[0,rb,D],[0,rb,-D]],P.roofDark,b);
    }
    box(0,0,hd+DECAL,DOOR,DOOR,DECAL,P.dark,b);
    for(const e of [-1,1])for(const zz of [-d*.25,d*.25])box(e*(hw+DECAL),h*.45,zz,DECAL,WINDOW,WINDOW,P.dark,b);
    box(0,h*.45,-hd-DECAL,WINDOW,WINDOW,DECAL,P.dark,b);
    const chimneyX=W*.45,chimneyTop=et+W-chimneyX+1.5;box(chimneyX,h,-1,1.5,chimneyTop-h,1.5,P.wall,b);
  }

  return {T,P,WP,DECAL,box,poly,tapered,cutBox,flat,ground,tree,village};
};
