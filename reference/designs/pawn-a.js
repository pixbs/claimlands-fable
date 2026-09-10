// Approved original marker at local origin, in world pixels.
window.ReferenceModel=function(kit){
  const {T,P,DECAL,box,tapered,cutBox,flat}=kit,g=new T.Group();
  cutBox(0,0,0,5.6,1,4.2,1,P.dark,g);
  tapered(0,1,0,4.2,3.5,2.8,2.8,3.2,P.faction,g);
  cutBox(0,4.2,0,2.8,1.8,2.8,.6,P.skin,g);
  // Skin ends at y=6, where the cap begins without a coincident shell.
  cutBox(0,6,0,2.8,1,2.8,.6,P.roofDark,g);
  box(0,4.2,-1.4-DECAL,1.6,1.8,DECAL,P.roofDark,g);
  return g;
};
