// Approved original marker at local origin, in world pixels.
window.ReferenceModel=function(kit){
  const {T,P,DECAL,box,tapered,cutBox,flat}=kit,g=new T.Group();
  cutBox(0,0,0,5.6,1,4.2,1,P.dark,g);
  tapered(0,1,0,4.2,3.5,2.8,2.8,3.2,P.faction,g);
  cutBox(0,4.2,0,2.8,2.8,2.8,.6,P.metal,g);
  const shield=flat([[-1.4,2.1],[1.4,2.1],[1.4,-.7],[0,-2.1],[-1.4,-.7]],P.faction,g);
  shield.position.set(-2.5,3,2.3);shield.rotation.y=20*Math.PI/180;
  const band=flat([[-.5,1.4],[.5,1.4],[.5,-.5],[0,-1.2],[-.5,-.5]],P.wall,shield);band.position.z=DECAL*3;
  box(0,5.2,1.4+DECAL,2.8,1,DECAL,P.dark,g);
  return g;
};
