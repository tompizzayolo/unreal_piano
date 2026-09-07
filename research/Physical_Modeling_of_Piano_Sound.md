# Physical Modeling of Piano Sound
Haifan Xie∗
September 24, 2024

## Abstract
This paper aims to develop a comprehensive physical model and numerical simulation

schemes for a grand piano. The model encompasses various subsystems, including hammer
felt, hammer shank, string, soundboard, air and room barriers, each modeled in three di-
mensions to approach their realistic dynamics. A general framework for 3D elastic solids
accounting for prestress and prestrain is introduced, particularly addressing the the non-
linearities arising from the large deformation of piano strings and the one-sided nature of
hammer felt-string contact. The study also examines coupling between subsystem through
mechanisms of surface force transmission and displacement/velocity continuity. To facilitate
numerical simulations, strong PDEs are translated into weak ODEs via a flexible space dis-
cretization approach. Modal transformation of system ODEs is then employed to decouple
and reduce DOFs, and an explicit time discretization scheme is customized for generating
digital audio in the time domain. The study concludes with a discussion of the piano model’s
capabilities, limitations, and potential future enhancements.

## Contents
# 1 Introduction                                                                              2

# 2 Preliminary: 3D linear elastic solid with prestress                                       3
2.1 Strong form (PDEs) . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .    3
2.2 Weak form (ODEs) . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .    5

# 3 Model for piano soundboard                                                                 7

# 4 Model for piano strings                                                                    8

# 5 Model for sound radiation in the air                                                      9
5.1 Model for the air . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . 9
5.2 Model for the room . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . 12

# 6 Model for coupling between different parts of the piano                                   13
6.1 Model for hammer-string coupling . . . . . . . . . . . . . . . . . . . . . . . .      13
### 6.1.1 One hammer striking one string . . . . . . . . . . . . . . . . . . . . . .      13
### 6.1.2 One hammer striking two or three strings . . . . . . . . . . . . . . . .        15
6.2 Model for string-soundboard coupling . . . . . . . . . . . . . . . . . . . . . .      17
### 6.2.1 Static coupling . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .   18
### 6.2.2 Dynamic coupling . . . . . . . . . . . . . . . . . . . . . . . . . . . . .      18
### 6.2.3 More discussions on coupling . . . . . . . . . . . . . . . . . . . . . . .      19
6.3 Model for soundboard-air and room-air coupling . . . . . . . . . . . . . . . .        20

# 7 Numeric schemes                                                                           20
7.1 Modal superposition method for solving coupled ODEs . . . . . . . . . . . . .         20
### 7.1.1 Modal transformation of second-order ODEs . . . . . . . . . . . . . .           20
### 7.1.2 Modal transformation of first-order ODEs . . . . . . . . . . . . . . . .        22
7.2 Explicit time discretization for coupling between systems . . . . . . . . . . .       23
### 7.2.1 Time stepping of second-order ODEs . . . . . . . . . . . . . . . . . . .        24
### 7.2.2 Time stepping of first-order ODEs . . . . . . . . . . . . . . . . . . . .       25
∗ Third-year master student, School of Accounting, Guangdong University of Foreign Studies, Guangzhou,

China. Email: fan455@outlook.com.

### 7.2.3 Time stepping of hammer shank rotation ODE . . . . . . . . . . . . .            25
7.3   Concatenating the whole model . . . . . . . . . . . . . . . . . . . . . . . . . .     26
### 7.3.1 Space discretization . . . . . . . . . . . . . . . . . . . . . . . . . . . .    26
### 7.3.2 Time discretization . . . . . . . . . . . . . . . . . . . . . . . . . . . . .   26

# 8 Conclusion                                                                                   29

A Prestrain and prestress                                                                      32

B Lagrangian formulation                                                                       34

C Elastic material contacting rigid surface in 3D setting                                      35

D An energy-stable scheme for nonlinear forces                                                 37

# 1 Introduction
This study aims to present a detailed physical model of a grand piano and its numeric imple-
mentation schemes. The main features of our model and numeric schemes include:
• As 3D as possible. Subsystems including hammer felt, hammer shank, soundboard, air
and room barriers are all fully 3D geometrically modeled. This also means unless for rigid
bodies (hammer shank), their 3-directional displacements fully vary with their 3D positions.
Our geometric model of piano strings is semi-3D, viz. a cylinder geometry described by
coordinates of one axis, but accounts for the strings’ 3-directional displacements.
• A general framework for 3D elastic solid, from both Newtonian and Lagrangian perspec-
tives. Rooted in the 3D elasticity theory, this framework not only follows the classic
stress-strain relationship, but also introduces a simple and intuitive model for prestrain
and prestress. For significantly prestressed structures like piano strings, we offer a more
“naturally linear” prestress model compared with the elaborated geometrically exact non-
linear stiff string model in [26]. For nonlinearity due to large deformation, we present a time
discretization scheme based on the scalar auxiliary variable (SAV) method [29]. For non-
linearity due to collision, we present a time discretization scheme based on the master-slace
approach.

• Consideration of coupling between subsystems of the piano. Two mechanisms are consid-
ered here: surface force transmission and displacement/velocity continuity. Furthermore,
nonlinear coupling due to collision rather than fixation is considered in the interaction be-
tween hammer felt and string, and discussed (but failed to implement) in the interaction
between string and two bridge pins.

• A straightford framework for weak forms and system ordinary differential equations (ODE).
For the sake of numeric simulation via the finite element method (FEM), we transform the
strong form partial differential equations (PDE) into first or second order system ODEs for
each subsystem of the piano. This is achieved via a flexible space discretization framework,
which can easily incoporate Dirichlet boundary conditions.

• Modal transformation for solving system ODEs. By solving generalized eigenvalue problems
of the mass and stiffness matrices, we decouple the coupled system ODEs and significantly
reduce the large vector of degrees of freedom (DOF) into a much smaller vector of modal
DOFs.
• An explicit time discretization scheme customized for solving modal ODEs exhibiting source
term coupling. This scheme is relatively efficient in that no mid steps are evaluated and
no inversion of large non-diagonal matrices is needed. This scheme is relatively accurate
in that the rhs source terms of the next time step are used for approximate integration
whenever possible, and more than one iteration of numeric integration at each time step
may be run to improve convergence.

Figure 1: The 9 surface forces acting on the 3 positive sides of an infinitesimal volume

The rest of this paper starts from a basic 3D elastic prestressed solid model in section 2, which will
be applied in several cases later. Subsequently sections 3 to 5 introduce models for subsystems
of the piano: soundboard, string, air and room barriers. Section 6 investigates the mechanisms
for coupling between these subsystems, where hammer felt and shank models are introduced.
Section 7 presents the modal superposition method and explicit time stepping schemes to solve
the derived system ODEs numerically, and a comprehensive computation procedure for running
the simulation. Section 8 gives a brief summary of the whole model and discusses current
limitations and future outlooks.
Here we explain some notation patterns that will appear throughout this paper. If not ex-
plicitly specified, symbols defined only apply to the current section. In cases where avoiding
symbol conflict is necessary, superscripts (a) , (b) , (c) , (d) , (e) ,(f ) refer to the string, soundboard,
air, hammer shank, hammer felt parts, room barriers of a piano physical system respectively.
For the convenience of notation, (x, y, z) and (x1 , x2 , x3 ), (u, v, w) and (u1 , u2 , u3 ) are used inter-
changeably. Bold letters represent vectors or matrices while non-bold letters represent scalars.
Definitions for scalars, vectors and matrices with subscript 0 , if not explicited stated, are auto-
matically inferred from their counterparts without subscript 0 .

# 2 Preliminary: 3D linear elastic solid with prestress
2.1     Strong form (PDEs)
In this study, the physical models of piano strings and soundboard are based on the full or
reduced versions of the linear theory of elasticity [1]. In a 3D Cartesian coordinate system
(x, y, z), consider a material defined over a space Ω ⊂ R3 with a boundary Γ ⊂ R3 , with
homogenous or heterogenous density ρ(x, y, z), and dynamic displacements in the 3 directions as
⊤
u(x, y, z, t) = [u(x, y, z, t), v(x, y, z, t), w(x, y, z, t)] .                (2.1)

Based on the displacement field, we shall perform a force analysis of the material, considering
two kinds of forces: surface force and body force.
Stress is a major source of surface force for elastic material, and can be derived from the
stress-strain relation. The strain matrix (second-order symmetric tensor) is expressed by the
gradients of displacements as
# 1 1
                                                                         
ϵ11 ϵ12 ϵ13                 ∂x u       2 (∂y u + ∂x v)   2 (∂z u + ∂x w)
ϵ =  ϵ12 ϵ22 ϵ23  =  12 (∂y u + ∂x v)            ∂y v        1
# 2 (∂z v + ∂y w)
,  (2.2)
# 1 1
ϵ13 ϵ23 ϵ33           2 (∂z u +  ∂x w) 2 (∂z v +  ∂ y w)       ∂z w

which is the linearized Green-Lagrange strain. It can also be written in vector form as
                        
ϵ11             ∂x u
 ϵ22            ∂y v    
                         X   3
→
−      ϵ33  
          ∂z w    
ϵ =        =                 =      H i ∇ui ,                        (2.3)
 2ϵ12   ∂y u + ∂x v  i=1
                  
 2ϵ13   ∂z u + ∂x w 
2ϵ23         ∂z v + ∂y w
where                                                                         
# 1 0   0            0      0   0            0     0   0
 0      0   0          0      1   0          0     0   0 
                                                         
 0      0   0 
 , H2 =  0      0      , H3 =  0
# 0                 0   1 
                       
H1 = 
 0
.            (2.4)
        1   0 

 1
        0   0 

 0
       0   0 

 0      0   1          0      0   0          1     0   0 
# 0 0   0            0      0   1            0     1   0
From the generalized Hooke’s law, the stress-strain relationship is (here the stress is Cauchy
stress)                                                                     
σ11                    D11 D12 D13 D14 D15 D16
 σ22                          D22 D23 D24 D25 D26 
                                                           
→
−      σ33 
      →
−                        D33 D34 D35 D36 
σ =  
=Dϵ, D=
                                       ,         (2.5)
 σ12                                      D44 D45 D46     
 σ13                                             D55 D56 
σ23                   Sym.                              D66
and the stress matrix (second-order symmetric tensor) is defined as
                 
σ11 σ12 σ13
σ =  σ12 σ22 σ23  .                                         (2.6)
σ13 σ23 σ33
As shown in figure 1, there are 9 stress forces acting on the 3 positive surfaces (as well as 9 stress
forces on the 3 negative surfaces not shown) of an infinitesimal volume of material. D is called
the constitutive matrix and its inverse D −1 is called the compliance matrix. The i th row or
column of the stress matrix can be expressed as
                              
X3                                       A1          A11 A12 A13
⊤
σi =      Aij ∇uj , Aij = H i DH j , A =  A2  =  A21 A22 A23  ,                    (2.7)
j=1                                      A3          A31 A32 A33
where block matrix A is symmetric. As σ i represents surface forces, converting it to body force
would result in ∇ · σ i .
Prestress is considered as the initial stress of material in static equilibrium, which is not
included in the above analysis of stress. Incorporating prestress into our analysis would require
some additional work as presented in appendix A, which we shall refer to. Define the static
tension field matrix (second-order symmetric tensor) and its vector form as
      
T11
                                 T22 
T11 T12 T13                           
→
−             T23 
T (x, y, z) =  T12 T22 T23  , T (x, y, z) =   T12  ,
                 (2.8)
T13 T23 T33                     
 T13 


T23
which can be visualized by figure 1 similar to stress. Since T is the prestress in static equilibrium,
∇ · T = 0 should hold (ignoring gravity). As per (A.7), the contribution of prestress to the total
stress consists of the static prestress T , and the dynamic prestress τ which is also a symmetric
tensor. Denote T i and τ i as the i th row or column of T and τ respectively, then we find
                                 
X3                                                        B1          B 11 B 12 B 13
τi =      B ij ∇uj , B ij = Ψ0 H ⊤  ⊤ ⊤
i Ψ1 Ψ2 DΨ2 Ψ1 H j , B =
 B 2  =  B 21 B 22 B 23  ,
j=1                                                       B3          B 31 B 32 B 33
(2.9)

where block matrix B is symmetric; the scalar Ψ0 and symmetric matrices Ψ1 and Ψ2 , all
constant w.r.t. unknowns, are defined in appendix A. Similar to σ i previously discussed, T i and
τ i contain the 3 sources of prestress in the same i th axis. For static equilibrium, ∇ · T i = 0
should hold; then in dynamic states, ∇ · τ i is the body force of prestress in the i th axis.
Besides stress and prestress which are conservative, non-conservative forces like damping force
often exist. For elastic non-metallic solid, viscoelastic damping is often the predominant damping
[15]. We hereby adopt a simple viscoelastic model: the viscous damping force is positively
proportional to the first-order time derivative of stress and prestress1 . Occurring on surfaces of
small volumes, the damping forces can be visualized by figure 1 similar to stress. The vector of
damping forces parallel to the xi axis is defined as

ς i = 2µ∂t (σ i + τ i ),                                     (2.10)

where 2µ > 0 is the damping coefficient. Other damping models can also be incoporated into our
model, e.g. structural damping that adds imaginary parts to elasticity coefficients [3]2 . As for
non-conservative forces besides damping, we represent them as a single force F = [F1 , F2 , F3 ]⊤
and will investigate them for different physical systems later.
Having derived all the relevant forces acting on an infinitesimal volume of solid material, we
can invoke Newton’s second law to derive 3 PDEs as

ρ∂tt ui = ∇ · Gi + Fi , i = 1, 2, 3.                                 (2.11)

where vector Gi = σ i + τ i + ς i is defined as the i th row or column of symmetric tensor G
representing all surface forces except the static prestress. The above equation can also be derived
via a Lagrange formulation of the virtual work and the variations of kinetic and potential energy.

2.2     Weak form (ODEs)
We then seek for a weak form of (2.11) via variational formulation. Multiplying an arbitrary test
function vector ψ(x, y, z) on both sides of it and integrating over Ω yields
Z                Z                 Z
ψρ∂tt ui dV −     ψ∇ · Gi dV =     ψFi dV                    (2.12)
ZΩ                ZΩ                ZΩ
ψρ∂tt ui dV +     (∇ψ) Gi dV =      ψFi dV                    (2.13)
Ω                    Ω                      Ω

where dV = dxdydz and applying the gradient operator ∇ to a vector results in its Jacobian
matrix. In the above formulation, integration by parts is utilized with Neumann boundary
condition imposed as
Gi · dΓ = 0,                                  (2.14)
where dΓ is the outward normal vector of the tangent plane of any point on Γ. Note that
this condition applies only when the test function is non-zero on the boundary, viz. Neumann
boundary conditions need not be satisfied at points where Dirichlet boundary conditions are
present.
We now use space discretization and the Galerkin method to transform (2.13) into second-
order ODE. Based on (2.1), define the displacement field of the soundboard as u∗ ≈ u = [u, v, w]⊤
(here superscript ∗ means the exact solution) as

u(x, y, z, t) = φ1 (x, y, z) · [S 1 ξ(t) + S 0,1 ξ 0 (t)] ,
v(x, y, z, t) = φ2 (x, y, z) · [S 2 ξ(t) + S 0,2 ξ 0 (t)] ,
w(x, y, z, t) = φ3 (x, y, z) · [S 3 ξ(t) + S 0,3 ξ 0 (t)] ,                   (2.15)
# 1 An explanation for this is that stress and prestress, along with viscous damping force, are resistance to

deformation, and so their relations should be positive.
# 2 The original real consitutive matrix is symmetric. If we add imaginary values to it, it may be beneficial to

make it an Hermitan matrix. This way energy may be dissipated to the imaginary part of the system but not
outside the system, i.e. energy is still conserved within the system.

where the sizes of vectors and matrices are

ξ : N × 1, ξ 0 : N0 × 1,
φ1 : N1 × 1, S 1 : N1 × N, S 0,1 : N0,1 × N0 ,
φ2 : N2 × 1, S 2 : N2 × N, S 0,2 : N0,2 × N0 ,
φ3 : N3 × 1, S 3 : N3 × N, S 0,3 : N0,3 × N0 .                        (2.16)

In (2.15), each scalar displacement unknown is expressed as the dot product of a space function
vector and time function vector. In FEM, φi is the vector of shape functions (also known as
interpolation functions) for variable ui , and ϑi = S i ξ+S 0,i ξ 0 is the vector of coefficients of shape
functions, often interpreted as nodal displacements. The only unknown here is ξ(t), the vector of
unknown DOFs, whereas other vectors and matrices are known. ξ(t) is defined so that through
some linear transformation by S 1 and S 0,1 ξ 0 (t), the nodal displacements ϑi can be obtained.
The reason we do not simply define ui = φi · ξ i but consider a linear transformation is that it
provides additional flexibility when imposing Dirichlet boundary conditions. For instance, when
u + v rather than u or v is known to be some nonzero functions on the space boundary, ξ only
needs to contain u and u + v is incorporated in ξ 0 . We can thus see that total DOF, viz. the
number of time functions we need to solve, is N that does not necessarily equal to N1 + N2 + N3 .
Often in simple cases, matrix S = [S ⊤       ⊤     ⊤ ⊤
# 1 , S 2 , S 3 ] is diagonal with many ones and some zeros on
the diagonal.
To express the displacement and its gradient in the DOF vector, we find the below relations:

ui = P i ξ + P 0,i ξ 0 , u = P ξ + P 0 ξ 0 , ∇ui = Qi ξ + Q0,i ξ 0 ,
         ⊤                                 ⊤             
P1              φ1 S 1               P 0,1        φ1 S 0,1
P =  P 2  =  φ⊤        2 S2
 , P 0 =  P 0,2  =  φ⊤ 2 S 0,2
,
P3                ⊤                  P 0,3         ⊤
φ3 S 3                            φ3 S 0,3
                       ⊤
                               ⊤

Q1              (∇φ1 ) S 1                 Q0,1         (∇φ1 ) S 0,1
Q =  Q2  =  (∇φ2 )⊤ S 2  , Q0 =  Q0,2  =  (∇φ2 )⊤ S 0,2  .                        (2.17)
                                                       
Q3                     ⊤                   Q0,3                 ⊤
(∇φ3 ) S 3                              (∇φ3 ) S 0,3

Combining (2.17) with (2.7) (2.9) , the following relations are found:

σ i = Ai Qξ + Ai Q0 ξ 0 ,
τ i = B i Qξ + B i Q0 ξ 0 ,                               (2.18)

Now we subsitute (2.17) (2.18) into (2.11) (2.12) (2.13), and use P ⊤
i as a vector of test functions
for the i th PDE. These would yield 3 groups of weak-form equations, each group having N
equations. Summing the 3 groups into 1 group, we derive a system of N coupled second-order
ODEs as
M ξ̈(t) + C ξ̇(t) + Kξ(t) = f (t),                           (2.19)
where
Z                        Z
M=          ρP ⊤ P dV, M 0 =
ρP ⊤ P 0 dV,
ZΩ                Ω
Z
⊤
K=    Q (A + B) QdV, K 0 =    Q⊤ (A + B) Q0 dV,
Ω                                  Ω
C = 2µK, C 0 = 2µK 0 ,
f (t) = f 0 (t) + f 1 (t),
Z
f 0 (t) = −M 0 ξ̈ 0 − C 0 ξ˙0 − K 0 ξ 0 , f 1 (t) =       P ⊤ F dV            (2.20)
Ω

are called the mass matrix, stiffness matrix, damping matrix and force vector respectively. In
the next following sections, we shall apply the above 3D elastic solid material model to various
parts of the piano physical system, including soundboard, string, room material and hammer
felt.

Figure 2: The piano soundboard in top view (left) and side view (right)

# 3 Model for piano soundboard
The study treats the grand piano soundboard as a multi-layer plate with irregular geometry,
and physically models it as a 3D structure. In [10, 11, 13], the soundboard was modeled as a
Reissner-Mindlin plate, accounting for the ribs and bridges by making the thickness, density,
and elastic coefficients position-dependent; however, it is unclear how the different orthotropic
angles of each layer were handled. In [33], the soundboard was modeled as a Kirchhoff-Love
plate, considering a 90 degrees orthotropic rotation of the ribs; however, as the Kirchhoff-Love
plate specifies only 1 unknown for the 3D displacement field, it may not be accurate enough,
especially regarding the ribs and bridges which are much thicker than the board. In [34, 25],
the soundboard was modeled as a 3D structure using tetrahedral elements, then reduced it into
a modal model; this is an advancement from 2D to 3D soundboard, but the authors have not
presented much details regarding the 3D equations. Moreover, some features of the soundboard
seem still under-evaluated: the prestress exerted by the string on the soundboard; the role of
rim and lid in the soundboard’s vibration; the more exact 2D Dirichlet boundary conditions for
the 3D soundboard, different from the case of 1D boundary conditions for 2D soundboard.
To better account for the soundboard’s multi-layer feature, the current study implements a
fully 3D soundboard model. Figure 2 shows a simplified soundboard sketch based on [23]. In our
physical model, the soundboard is an entity composed of 5 parts: board, ribs, bridges, rim, lid.
The board is parallel to the xOy plane, with the ribs attached to its under side and the bridges
attached to its upper side. The horizontal fibers of the board and bridges are approximately
parallel, while the horizontal fibers of ribs are almost orthogonal to that of the board. The rim
consists of 2 parts: the inner rim connects the board’s boundary from below using bolts and
dowels, and the outer rim encases the board and the lower rim. The lid, as large as the board
and with an angle to it, is an extension of the outer rim through a wooden stick (considered
part of the lid) and hinges. All parts of the soundboard are treated as a whole in computation,
meaning that the displacements of part intersections are uniform.
The soundboard’s governing PDEs follow (2.11) and ODEs follow (2.19), so we do not need to
present them again. We only need to discuss some parameters, initial conditions and boundary
conditions here. Define the occupied space of soundboard as Ω ⊂ R3 , and the boundaries of
soundboard as Γ1 , Γ2 ⊂ Γ ⊂ R3 . Here Γ contains all the 2D surfaces of the 3D volume of
soundboard; Γ1 contains only the underside surfaces of the inner and outer rims, as marked red
at the right of figure 2; Γ2 is Γ excluding Γ1 , the vibrating parts. We shall see in the following
that Γ1 and Γ2 are where the Dirichlet boundary conditions apply to the soundboard and the
air respectively.
Given that all parts of the soundboard are mainly made of wood, an orthotropic material

Figure 3: The piano string in 3D view

whose orthotropic directions are determined by fibers [7], the compliance matrix writes
ν
− Exyx   − νExzx    0     0     0
                                              
Ex
# 1 ν

             Ey      − Eyzy     0     0     0 
# 0 0     0 
D −1

orth =                          Ez                    ,                  (3.1)

                               Gxy    0     0 
                                     Gxz    0 
Sym.                                   Gyz

where E, G, ν are the young’s modulus, shear modulus and Poisson’s ratio. Note that different
layers would have different elastic coefficients. It is then necessary to consider that for a certain
layer like ribs, the global axes x, y may need to be rotated by an angle α, as denoted in figure
2, to become the material orthotropic axes x′ , y ′ and for a certain layer. The actual constitutive
matrix is then
C2       S2 0
                                           
SC       0    0
 S2           C2 0       −SC       0    0 
                                           
⊤
      0        0    1      0       0    0 
D = Z D orth Z, Z =                           2     2
,           (3.2)
 −2SC 2SC 0 C − S                  0    0 
      0        0    0      0       C S 
# 0 0    0      0      −S C

where S = sin α, C = cos α. Readers may refer to [27] for a detailed deduction. After rotation,
the constitutive matrix would have 13 non-zero entries for its upper triangular part.
As for initial and boundary conditions, equation (2.11) is constrained by:

u|x∈Γ1 = 0,                                          (3.3)

u|t=0 = ∂t u|t=0 = 0.                                    (3.4)
The choice of this boundary condition stems from that the whole soundboard is hard supported
by beams and legs on the under side of rim (see figure 2). Thanks to the flexiblility of our 3D
model, the rim can be treated as a natural extension of the main board, thereby capturing more
nuanced boundary conditions compared to the clamped or simply supported cases in 2D models.
Given approriate boundary conditions, the mass and stiffness matrices are symmetric positive
definite.

# 4 Model for piano strings
The symbols in this section follow sections 2 and 3 if not explicitly defined. Definitions for
scalars, vectors and matrices with subscript 0 , if not explicited stated, are automatically inferred
from their counterparts without subscript 0 .
The study treats a grand piano string as a cylinder material, and physically models it as a
1D beam prestressed longitudinally. As shown in figure 3, the string is fixed at the agraffe end
(immobile) and coupled to the bridge end (mobile) [9] through pins. Define that the central line
of string range start from the tunning pin [−L0 , 0, 0]⊤ , then go through the agraffe x0 = [0, 0, 0]⊤ ,
the front bridge pin x1 = [L1 , 0, 0]⊤ , the rear bridge pin x2 = [L2 , 0, 0]⊤ , and finally the hitch pin

Figure 4: The piano in a room

x3 = [L3 , 0, 0]⊤ , where L1 is the so-called “speaking length”; let r be the radius of cross-section
and ρ be the homogenous density per unit volume. It is known that the vibration of piano strings
is primarily vertical (z direction) [4]. Nevertheless, logitudinal vibration (x direction) [5] and
horizontal vibration (y direction) [31] may also be essential, as they contribute respectively to
the sound precursor [9] and double-decay [32] phenomena. Two more kinds of vibration that
require attention are the rotations of the string’s cross-section towards the y and z axes, so as
to account for the stiffness of piano strings that may result in slightly inharmonic sounds [10].
To incorporate all these essential kinds of vibrations, define the displacement field of a piano
string as u∗ ≈ u = [u, v, w]⊤ in a similar notion to (2.15). In addition, we specify that
                           
φ11 (x)              S 11
φ1 (x, y, z) =  −yφ12 (x)  , S 1 =  S 12  ,
−zφ13 (x)              S 13
φ2 (x, y, z) = φ2 (x), φ3 (x, y, z) = φ3 (x),                           (4.1)

where subscripts 11 ,2 ,3 ,12 ,13 refer to vectors or matrices defined for longitudinal, horizontal, ver-
tical, horizontal rotational and vertical rotational vibrations respectively, as marked u, v, w, α, β
in figure 3. Definition for φ0,1 and S 0,1 are similar to the above. From a 3D perspective, it can
be seen that with regard to the y and z axes, the x displacement is first-order modeled, while the
y and z displacements are zeroth-order modeled. As the steel used to manufacture piano strings
can be considered as isotropic material, the elasticity coeffieicnts in (3.1) simplifies to

E = Ex = Ey = Ez ,
ν = νxy = νxz = νyz ,
E
G=           = Gxy = Gxz = Gyz .                                  (4.2)
2(1 + ν)

It now suffices to derive a system of N coupled second-order ODEs similar to (2.19) (2.20).
In order that reasonable solutions can be found, equation (2.11) is at least constrained by the
following Dirichlet boundary conditions:

u|x=0 = u|x=L3 = 0,                                       (4.3)

which means the motion of string vanishes at the agraffe point and the hitch point. Another
Dirichlet boundary condition at the coupling point x1 will be discussed in section 6. The initial
conditions are
u|t=0 = ∂t u|t=0 = 0.                                 (4.4)

# 5 Model for sound radiation in the air
5.1    Model for the air
The study adopts a 3D acoustic wave model to simulate piano sound radiation in the air. For
simulating wave propagation from the piano soundboard to a listener situated at a particular
location, [20] used Rayleigh integral to compute the acoustic pressure field, which is able to
simulate the different delays and decays of sound at different positions, but unable to account

for boundary conditions. [10, 11, 13] used 2 first-order linear acoustic equations for 4 coupled
unknowns: acoustic pressure (1 unknown) and acoustic velocity (3 unknowns). This approach
excels at capturing reflections at spatial boundaries like walls and soundboard-air coupling, but
seems unable to produce decaying room impulse response due to the absence of damping terms3 .
Fluid dynamics exhibit both linear and nonlinear behaviours as decribed by the Navier-Stokes
equation, but considering that sound pressure fluctuations in air are often small, linearized models
for sound radiation in the air can achieve a satisfactory level of accuracy for us. The acoustic
radiation equations we utilize are based on the linearized fluid dynamics equations [19], including
conservation of mass and momentum. It works for isotropic, compressible, viscous, adiabatic flow
with homogenous ambient states, in Eulerian description. Damping due to heat conduction is
simplified away here and readers may refer to [21, 22] for incorporating it. The governing PDEs
write
ṗ + ∇ · u = 0,                                        (5.1)
ρc2
ρu̇ = ∇ · G + F ,                                          (5.2)

where u(x, y, z, t) is the acoustic velocity field; p(x, y, z, t) is the perturbation of acoustic pressure
field, also should be the final digital audio signal; ρ is the air density; c is the sound propagation
speed in the air; G is the symmetric surface force tensor; the gradient operator ∇ applied to a
vector returns its Jacobian matrix, and the divergence operator ∇· applied to a matrix returns a
vector of each row’s divergence; F = [F1 , F2 , F3 ]⊤ is the external force (viz. body force) treated
as zero in this study. According to the Navier-Stokes equation, the surface force tensor G is
defined as
G = −pI + µ1 ∇u + (∇u)⊤ + µ2 (∇ · u)I,

(5.3)
where µ1 and µB are dynamic viscosity and bulk viscosity coefficients accounting for energy
dissipation, and µ2 = µB − 32 µ1 . To express the divergence of surface force using acoustic
velocity gradients, we find
X
∇ · Gi = −∇p + ∇ · ς i , ς i =             (µ1 Aij + µ2 B ij ) ∇uj ,
j=1
                                                                 
A1       A11             A12     A13            B1       B 11 B 12 B 13
A =  A2  =  A21             A22     A23  , B =  B 2  =  B 21 B 22 B 23  ,
A3       A31             A32     A33            B3       B 31 B 32 B 33
                                                                        
# 2 0 0 0 0               0   0    0 0            1 0 0 0 1 0 0 0 1
 0 1 0 1 0               0   0    0 0          0 0 0 0 0 0 0 0 0 
                                                                        
 0 0 1 0 0               0   1    0 0          0 0 0 0 0 0 0 0 0 
                                                                        
 0 1 0 1 0               0   0    0 0          0 0 0 0 0 0 0 0 0 
                                                                        
A=  0 0 0 0 2               0   0    0 0 , B =  1 0 0 0 1 0 0 0 1 ,                              (5.4)
                          
 0 0 0 0 0               1   0    1 0          0 0 0 0 0 0 0 0 0 
                                                                        
 0 0 1 0 0               0   1    0 0          0 0 0 0 0 0 0 0 0 
                                                                        
 0 0 0 0 0               1   0    1 0          0 0 0 0 0 0 0 0 0 
# 0 0 0 0 0               0   0    0 2            1 0 0 0 1 0 0 0 1

where Gi , ς i is the i th column or row of G, ς; here ς is the damping force tensor. The 2 PDEs
# 3 It seems the acoustic model in [11] relies on soundboard-air coupling to produce the damping.      When the
acoustic equations are fully coupled to the soundboard equations, the soundboard’s damping mechanisms may
apply to the air as well. This means even if the acoustic equations contains no damping, the overall energy may
be stll stable and waves with infinite amplitudes would not occur. However, we choose to add viscous damping
terms to the acoustic equations for two reasons. Firstly, it can be observed in audio recording that a very short
impulse in a room gets a decaying response, which attributes to the reverberation effect. Secondly, the coupling
computation approach we shall introduce later actually relies on decoupling and iterative strategies, meaning that
without the room itself’s damping mechanisms the computed pressure may exhibit infinite amplitudes.

(5.1) (5.16) then become4

ρu̇i − ∇ · ς i + ∂xi p = Fi (i = 1, 2, 3),                              (5.5)
ṗ + ρc ∇ · u = 0,                                                      (5.6)

where the divergence of velocity can be expressed in velocity gradients as
X
∇·u=             H j ∇uj ,
j=1
                                       
                            1 0 0            0   0   0   0   0   0
H=         H1     H2     H3       = 0 0 0            0   1   0   0   0   0 .            (5.7)
# 0 0 0            0   0   0   0   0   1

For acoustic modes the vorticity ∇ × u = 0 may be assumed zero so that a decoupled equation
containing only p as unknown may be obtained, but we choose to not do so because the velocity
field of any solid to couple with may not satisfy zero vorticity.
Now we seek for the weak forms of (5.5) (5.6) via the Galerkin method. Multiplying an
arbitrary test function vector ψ on both sides of (5.5) yields
Z              Z                    Z             Z
ψρu̇i dV −    ψ (∇ · ς i ) dV +    ψ∂xi pdV =    ψFi dV
Ω             Ω
Z             Z                  ZΩ            ZΩ
ψρu̇i dV +     (∇ψ) ς i dV +      ψ∂xi pdV =    ψFi dV.        (5.8)
Ω                 Ω                    Ω                   Ω

When using integration by parts in the above, Neumann boundary condition is imposed as

ς i · dΓ = 0,                                           (5.9)

where dΓ is the outward normal vector of the tangent plane of any point on the boundary of
acoustic space. This condition means the damping force should vanish on the non-Dirichlet
boundaries. Similarly for (5.6), multiplying a test function vector results in
Z            Z
ψ ṗdV +    ψρc2 (∇ · u) dV = 0.                  (5.10)
Ω              Ω

To derive the ODEs, we define space discretization similar to (2.15), but with one more variable
p, as

u(x, y, z, t) = φ1 (x, y, z) · [S 1 ξ(t) + S 0,1 ξ 0 (t)] ,
v(x, y, z, t) = φ2 (x, y, z) · [S 2 ξ(t) + S 0,2 ξ 0 (t)] ,
w(x, y, z, t) = φ3 (x, y, z) · [S 3 ξ(t) + S 0,3 ξ 0 (t)] ,
p(x, y, z, t) = φ4 (x, y, z) · [S 4 ξ(t) + S 0,4 ξ 0 (t)] .                  (5.11)

And similar to (2.17), we define convenience matrices to express that

u = P ξ + P 0 ξ 0 , ui = P i ξ + P 0,i ξ 0 , ∇ui = Qi ξ + Q0,i ξ 0 ,
        ⊤                                   ⊤             
P1            φ1 S 1                  P 0,1        φ1 S 0,1
P =  P 2  =  φ⊤     2 S2
 , P 0 =  P 0,2  =  φ⊤   2 S 0,2
,
P3              ⊤                     P 0,3         ⊤
φ3 S 3                               φ3 S 0,3
                      ⊤
                                 ⊤

Q1            (∇φ1 ) S 1                    Q0,1         (∇φ1 ) S 0,1
Q =  Q2  =  (∇φ2 )⊤ S 2  , Q0 =  Q0,2  =  (∇φ2 )⊤ S 0,2  ,
                                                       
Q3                    ⊤                     Q0,3                 ⊤
(∇φ3 ) S 3                                 (∇φ3 ) S 0,3
⊤                       ⊤
P 4 = φ⊤                ⊤
# 4 S 4 , P 0,4 = φ4 S 0,4 , Q4 = (∇φ4 ) S 4 , Q0,4 = (∇φ4 ) S 0,4 .                     (5.12)
# 4 It is also viable to decouple the acoustic equations into a single second-order equation containing only u as

unknown. We choose to not do so because it would result in a damping matrix not diagonalizable by the mass
or stiffness matrices. To fully decouple the system ODEs would then require doubling the DOFs (equivalent to 6
variables) to transform into a first-order system. This appears suboptimal to us as the original first-order system
of acoustic equations has only 4 varaibles.

Then similar to (2.18), we find
X
Aij ∇uj = Ai Qξ + Ai Q0 ξ 0 ,
j=1
X
B ij ∇uj = B i Qξ + B i Q0 ξ 0 ,
j=1

∂xi p = ∂xi φ4 · S 4 ξ + ∂xi φ4 · S 0,4 ξ 0 ,                (5.13)

so that space partial derivatives can be expressed as linear transformations of the DOF vector
ξ(t) or its time derivatives. It now sufficies to derive the system of second-order ODEs similar
to (2.19) (2.20). We use P ⊤  i as a vector of test functions for the i th weak form equation in
(5.8) and use P ⊤4 to test the  weak form (5.10), which would yield 4 groups of equations, each
group having N equations. Summing the 4 groups into 1 group, we derive a system of N coupled
first-order ODEs as
M ξ̇(t) + Kξ(t) = f (t),                             (5.14)
where
Z                     
M=           ρP ⊤ P + P ⊤
# 4 P 4 dV,
ZΩ                       
M0 =         ρP ⊤ P 0 + P ⊤
# 4 P 0,4 dV,
Z Ωh                                               i
K=          Q⊤ (µ1 A + µ2 B) Q + ρc2 P ⊤             ⊤
# 4 HQ + P Q4 dV,
ZΩ h                                                 i
K0 =         Q⊤ (µ1 A + µ2 B) Q0 + ρc2 P ⊤             ⊤
# 4 HQ0 + P Q0,4 dV,
ZΩ
f (t) =     P ⊤ F dV − M 0 ξ˙0 (t) − K 0 ξ 0 (t).                           (5.15)
Ω

Here M and K may not be as meaningful as the mass and stiffness matrices in the second-order
ODEs case. The initial conditions are

u|t=0 = 0, p|t=0 = 0.                               (5.16)

The acoustic space is often finite, which means boundary conditions are essential to shape
the solutions of acoustic equations. Let Γ(c) ∈ R3 be the acoustic space’s whole solid boundary,
(c)  (c)
of which Γ1 , Γ2 ∈ Γ(c) are the room boundary (grounds, walls, ceilings) and piano soundboard
parts respectively, as shown in figure 4. In [11], the walls were assumed rigid, which can produce
sound reflection effects but may miss sound absorption effects. This motivates our introduction
of vibratory acoustic boundaries to account for various phenomena like reflection, scattering,
absorption, diffusion, resonance etc.

5.2     Model for the room
In this section, we delve into the specification of room boundary models, which serve as the
cornerstone for the subsequent analysis of solid-air coupling dynamics. For the sake of con-
ciseness, we adopt a simplified representation of the room environment as a shoebox-shaped
3D space accommodating a piano. It is worth noting, however, that this room model remains
versatile and can be readily adapted to accommodate irregular room geometries. The room’s
boundaries—encompassing floors, walls, and ceilings—play pivotal roles in shaping the acoustic
behavior within, acting as both reflectors and absorbers of sound waves. In the context of physical
modeling, these boundaries are treated as 3D elastic materials, akin to the piano’s soundboard
model. This methodological alignment facilitates the straight application of the general 3D elas-
ticity model and the soundboard model to the room, obviating the need for reformulations of
the governing equations and weak forms.
The primary specialization within this framework lies in the stipulation of boundary condi-
tions of room barriers. As highlighted red in figure 4, the inner side of ground, wall and ceiling
(f )
(blue color), defined as Γ1 , is the room material’s interface with the air; the underside of ground

Figure 5: The piano hammer view from y (left) and x (right) direction

(f )
material (red color), defined as Γ2 , is where the Dirichlet boundary condition applies. Firmly
(f )
rooted in the earth, the ground material should experience zero displacement along Γ2 . This
condition is written as
u(f ) |x∈Γ(f ) = 0.                                (5.17)

In case the inner concrete layers of walls and ceilings are considered rigid, displacements on
the surfaces of them should be zero too. Another specialization is that prestress need not be
considered for room barriers. Even if it exists, the dynamic deformation may not be large enough
to induce significant effect of prestress on vibration.

# 6 Model for coupling between different parts of the piano
6.1     Model for hammer-string coupling
The piano hammer positioned below the string functions the excitation of string vibration, which
is known to be a highly nonlinear process [17]. We consider two main parts of the hammer
relevant to this excitation process: the wooden hammer shank that can be approximated as
non-deformable; the hammer felt that is deformable, impacting and exerting force on the string.
Normally, the hammer moves in a circle when triggered by piano player’s key action, striking
the string at certain point in during a very short period of time, and is also pushed back by the
vibrating string.

### 6.1.1 One hammer striking one string
For the case of one hammer striking one string, the study follows the 0D nonlinear hammer-
string interaction model in [11], with some refinements: for the case of one hammer striking one
string, the rotational movement of hammer [12] and the horizontal interaction force are taken
into account.
As shown in figure 5, xyz is the coordinate system of piano string, where the x axis of the
string system has an angle α0 to the ground. To better describe the hammer shank motion
which is rotation around point P1 , we will not often the use the xyz system. Instead, we define
a static coordinate system x′ y ′ z ′ with origin P1 , y ′ axis the same as y axis, z ′ axis with negative
direction the same as gravity, and x′ axis perpendicular to the y ′ O′ z ′ plane. The motion of
hammer shank as a rigid body can then be described as rotating around the y ′ axis and not
−−−→
moving in the y ′ direction. We can thus use a single variable θ(t), the angle of P1 P3 to the x′
axis, to fuuly describe the hammer shank motion. The hammer shank drives the overall motion
of the shank head P2 and felt head P4 . Nevertheless, when the felt head is in contact with
the string’s Q0 point, it undergoes a compression ϑ = [ϑ1 , ϑ2 , ϑ3 ]⊤ from P4 to P6 , which also
overlaps with the dynamic string point Q1 , which also contributes to its motion. To perform a
reasonable orthogonal decomposition of this compression, we define a dynamic coordinate system
−−−→
x′′ y ′′ z ′′ with origin P3 , x′′ axis with positive direction P5 Q0 , y ′′ axis with positive direction the

−−−→
same as y axis, and z ′′ axis with positive direction P5 P2 . The hammer compression can then be
decomposed onto the x′′ , y ′′ , z ′′ axes as marked as marked ϑ1 , ϑ2 , ϑ3 respectively in figure 5.
(a)                          (a)
Denote the displacement of string point Q0 with coordinate x0 in the xyz system as u0 =
(a)
u(a) (x0 , t). Denote the lengths of Q0 P1 , P0 P1 , P1 P3 , P2 P3 , P1 P2 , P2 P4 as L, L0 , L1 , L2 , L3 ,
−−−→
L4 respectively, and the angle of P1 Q0 to the x′ axis as α1 . Our geometric analysis finds the
coordinates of Q1 (equivalent to P6 in case of compression) in the x′ y ′ z ′ system and x′′ y ′′ z ′′
(a′ )        (a′′ )
system, denoted x1 and x1 , are
(a′ )                     (a)   (a′′ )    (a′ )
x1   = r 0 + R0 u0 , x1 = r + R(θ)x1 ,
                                       
L cos α1             cos α0 0 − sin α0
r0 =       0     , R0 =      0    1    0    ,
L sin α1             sin α0 0 cos α0
                                   
−L1                 cos θ 0 sin θ
r =  0  , R(θ) =          0     1   0 .                                   (6.1)
−L2                − sin θ 0 cos θ

(a′′ )            (a′ )
And converting x1             to x1            results in
(a′ )                            (a′′ )
x1   = s(θ) + S(θ)x1 ,
                                                                
L1 cos θ − L2 sin θ              cos θ               0   − sin θ
s(θ) =           0           , S(θ) =  0                   1      0    .             (6.2)
L1 sin θ + L2 cos θ              sin θ               0    cos θ

As previously discussed, we assume Q0 and P2 is non-deformable whereas P4 is deformable.
This means when the hammer is in contact with string, P4 moves to P6 ; when contact is absent,
the position of P4 is determined by the shank but not the string. The condition to satisfy that
(a′′ )
the hammer is in contact with string should be z1         ≤ L4 , viz. the z ′′ direction distance between
string point and shank head is less than the static thickness of hammer felt. The hammer felt
compression is then
(a′′ )           (a′′ )
(
x1 − L4 , z1              ≤ L4
ϑ(θ) =          (a′′ )                                                (6.3)
0, z0        > L4
where L4 = [0, 0, L4 ]⊤ ; negative value of ϑi means the hammer felt is compressed towards the
negative direction of x′′i axis, and vice versa. Due to compression, the hammer felt exerts an
′′            (a′′ )    (a′′ ) (a′′ )
interaction force F (a ) (θ) = [F1 , F2 , F3 ]⊤ (in x′′ y ′′ z ′′ system) on the string. Recipro-
′′
cally, the hammer felt suffers −F (a ) from the string according to Newton’s third law5 . Based
on [11], the hammer-string interaction force in the x′′i axis is modeled as a nonlinear function of
compression as
(a′′ )
Fi        = −sgn(ϑi ) [ki |ϑi |pi + ri ki ∂t (|ϑi |pi )] , i = 1, 2, 3, (6.4)
where ki is the stiffness of hammer felt; pi is a positive exponent accounting for nonlinearity;
ri is the relaxation coefficient accounting for the hammer’s hysteretic and dissipative behaviour;
function sgn(·) returns the sign of a real number. Converting the interaction force to x′ y ′ z ′ and
′             ′′
(a′ )
x′ y ′ z ′ coordinates yields F (a ) = S(θ)F (a ) and F (a) = R⊤
0F       .
To couple the hammer force with string motion, we need equations in the x′ y ′ z ′ system
governing the motion of hammer shank, a rigid body. Denote the coordinates of shank mass
(d′ )   (d′ )   (d′ )
centre P0 , shank rotation centre P1 , shank head P2 in the x′ y ′ z ′ system as x0 , x1 , x2
(d′ )                                         (d′ )
respectively. It can be found that x0 = [L0 cos(β0 + θ), 0, L0 sin(β0 + θ)]⊤ and x2 = s(θ),
−−−→        −−−→
where β0 is the angle of P1 P0 to the P1 P3 . Define the total mass and homogenous line density of
the hammer shank as m, ρ. It can be found that m = ρ(L1 + L2 ), and the axis-free position of
# 5 Note that we ignore here the string’s normal and tangent stress and prestress on the interaction surface

exerting on the hammer felt (as well as the relevant strain on the string side), which should already be zero per
the Neumann boundary condition specified in (2.14) if DOFs are given to string displacements on this surface.
Even if they should not be zero, it is relatively acceptable to ignore them as they are probably less contributive
than the hammer’s compression force. Treating them as non-zero would require imposing Dirichlet boundary
conditions on some relevant DOFs of the string’s side, which is a computational challenge.

center of mass P0 projected on P1 P3 and P2 P3 are ( 12 L21 + L1 L2 )/(L1 + L2 ) and 21 L22 /(L1 + L2 )
respectively. It is obvious that the shank as a rigid body suffers these forces: string reaction force
′′          ′′
F (d ) = −F (a ) at P2 ; a rotation constraint force at P1 , which has zero torque with respect to
the y ′ axis; gravity [0, 0, −mg]⊤ at P0 , where g is the scalar gravitational acceleration. It follows
′′
that the total torque with respect to the y ′ axis is mgL0 cos(β0 +θ)+l·F (d ) (θ) (only 1 dimension
of torque is needed here), where l = [L2 , 0, −L1 ]⊤ ; the moment of inertia with respect to the y ′′
axis, denoted I, is
Z L1          Z L2                                        
# 2 2     2
        1 3 1 3           2
I=         ρl dl +      ρ l + L1 dl = ρ       L + L + L1 L2                    (6.5)
# 0 0                       3 1 3 2

which does not depend on θ. Applying Newton’s second law for rotation, the differential equation
of hammer shank motion is
′′
I θ̈ = −µθ̇ + mgL0 cos(β0 + θ) + l · F (d ) (θ),                      (6.6)

where −µθ̇ is the simplified damping force. Here t = 0 is the moment when the felt head first
contacts the string and compression is still zero, and initial angle and angular velocity at this
time should be known.
From the aforementioned deductions, we can abstract the hammer-string interaction force
(a)
F (e) as a nonlinear operator F(u0 , θ) : (R3 , R) → R3 . This force should be added to the rhs of
(2.11) as a non-conservative force. Then, (2.11) and (6.6) forms 2 sets of coupled equations for
# 2 sets of variables (u(a) , θ), which provides a foundation for obtaining a solution theoretically.
Due to the highly nonlinear nature of hammer-string coupling, particularly in that Taylor series
approximation may not work well for potentially non-smooth functions in (6.4), it would be
inappropriate to use the common perturbation method. In section 7.2 we shall introduce an
explicit time discretization method to efficiently solve the hammer-string coupling.

### 6.1.2 One hammer striking two or three strings
The case of one hammer striking multiple, say three strings can simply be treated it as if three
independent felt heads were striking their corresponding strings. Three independent interaction
(e′ )
forces F i=1,2,3 are computed from three independent compressions. The string force exerted on
′        P3    (e′ )
the shank is the sum F (d ) = − i=1 F i , assuming the 3 compression forces apply to the same
point of shank head. However, this approach may lose the dynamic interaction between different
striking points of the hammer felt.
A 3D hammer model may be more accurate in capturing the interplay of different hammer
striking points and the different positions of compression forces. The hammer felt is now modeled
as a 3D elastic material with space discretization. Two kinds of spacial boundaries consist in the
hammer felt: one contains the felt’s 3 potential contact points with the 3 strings, represented as
(e′′ )                                 (e′′ )
P4,i with coordinates x4,i=1,2,3 ; another is the contact surface Γ1 ∈ R2 with the hammer shank.
The hammer shank is considered as a rigid 3D object, which means no inner space discretization
is needed for it. Above all, one needs to pay attention to the choice of coordinate system for
the 3D geometry of felt. A recommended choice here is the dynamic x′′ y ′′ z ′′ system, which
eliminates the felt’s rigid body movement component and is thus suitable for FEM computation.
′′    ′′                            ′′ ′′
Also, displacement fields u(e ) (x(e ) , t) for hammer felt and u(d ) (x(d ) , t) for hammer shank
need to be defined.
For more realistic modeling that the contact is dynamically distributed over a region rather
than occurring at only some dimensionless points, readers can refer to appendix C which de-
scribes an iterative algorithm for contact problems directly integrated into FEM, supporting
time-varying contact boundary and boundary coupling. Another more analytical (but also seems
more simplified) approach for contact problems was introduced in [35], where additional dynamic
variables including pressure, density, stiffness and damping of the contact surface along with their
governing equations are formulated.
Derivation of the felt compression dynamics is now based on the 3D elastic material model
and soundboard model previously introduced, without the need for modeling prestress. We first
consider the boundary conditions needed for the felt model. Denote the coordinate of Q1,i in
(a′′ )
the x′′ y ′′ z ′′ system as x1,i , which can be computed similar to (6.1). To represent hammer felt

compression, Dirichlet boundary condition should be imposed on these 3 contact points as
(a′′ )     (e′′ ) (a′′ )
(
′′
(e )   (e ′′
)      x1,i − x4,i , z1,i ≤ L4
u     (x4,i , t) =        (a′′ )               ,                 (6.7)
0, z1,i > L4

(a′′ )
where z1,i ≤ L4 means contact is present and otherwise not. This determined displacement
would occur on the rhs of system ODEs like (2.19) as a source term. It is assumed without
contact, not only P4,i but also the entire felt will not experience deformation; with contact, the
position of P4,i , i.e. P6,i , is the same as Q1,i , triggering motion and compression of the hammer
felt. However, this assumption is a simplification that when the hammer felt leaves the string,
its compression immediately recovers, and may result in a problem that Neumann boundary
condition is not satisfied at P4,i when there is no contact. For a more accurate treatment of
the contact boundary problem, readers can go to appendix C. There we describe an elastic-rigid
3D contact algorithm able to account for the time-varying contact surface and the time-varying
switch between Dirichlet and Neumann boundary conditions, but with increased algorithmic
(e′′ )
complexity and numeric convergence uncertainty. As for another boundary Γ1 which interfaces
the shank, Dirichlet boundary condition should be imposed on as
′′    ′′                    ′′       (e′′ )
u(e ) (x(e ) , t) = 0, x(e ) ∈ Γ1                ,                 (6.8)

because the rigid shank should have no deformation.
We then consider the dynamics of string and shank impacted by the felt. At the contact
′′   (e′′ )
point with string, the felt’s nonzero surface forces G(e ) (x4,i , t) should be transmitted to the
string as
                          
# 0 Z12 Z13         1
(a)
F i = −  Z12 Z22 Z23   1  ,
Z13 Z23 Z33           1
′′   (e′′ )
{Zij } = R⊤     ⊤ (e )
# 0 R(θ) G     (x4,i , t)R(θ)R0 ,                                 (6.9)

where Z11 is not transmitted considering that the hammer felt seems to have no direct contact
with the string’s surface whose normal is in the x axis (longitudinal direction). As for the felt’s
(e′′ )   (d′′ )
surface force exerting on the shank along the boundary Γ1 = Γ1 , previous practice of directly
transmitting the felt-string interaction force to the shank is not applicable here due to the 3D
′′       ′′     (e′′ )        ′′    ′′
nature of felt and shank. For a point x(e ) = x(d ) in Γ1 , with n(e ) (x(e ) ) as the outward
normal of its tangent plane, the felt would exert a surface force
′′      ′′                ′′        ′′         ′′         ′′
F (d ) (x(d ) , t) = −G(e ) (x(e ) , t)n(e ) (x(e ) )                         (6.10)

on the shank. Then the total contribution of felt force to the torque of shank with respect to the
y ′′ axis is
Z                                         
(d′ )   (d′ )          (d′ )
T1 (θ) =    ′
E 0 F       (x      , t) · x         dV
(d )
Γ
Z 1      h                                                      i
(d′′ )    (d′′ )                      (d′′ )
=          ′′
E 0 S(θ)F        (x        , t) ·   s(θ) + S(θ)x          dV
(d )
Γ
Z 1      h                                       i
(d′′ )   (d′′ )                (d′′ )
=          ′′
F        (x       , t) ·   l + Ex          dV,                      (6.11)
(d )
Γ1

where                                                                                              
L2           0                  0        −1         0                    0 1
l =  0  , E0 =  0                  0         0 ,E =  0                    0 0    (6.12)
−L1           1                  0         0         −1                   0 0
are constant quantities. Note that the only dependence of T1 on θ, though not explicitly written,
′′     ′′
consists in F (d ) (x(d ) , t), tracing back to the conversion from xyz to x′′ y ′′ z ′′ for string displace-
ments. The total contribution of gravity to the torque of shank with respect to the y ′′ axis is

Figure 6: The piano bridge view from 3 directions

still mgL0 cos(β0 + θ) if no relevant modifications are made when switching to the 3D hammer
model. The moment of inertia with respect to the y ′′ axis, constant through time, is
Z
ρ x2 + z 2 dV,

I=                                                  (6.13)
Ω(d′ )
′
where Ω(d ) ∈ R3 is the shank’s volume domain. Applying Newton’s second law for rotation, the
differential equation of hammer shank motion is

I θ̈ = −µθ̇ + mgL0 cos(β0 + θ) + T1 (θ).                         (6.14)

Now the whole model of 3D hammer-string coupling has been established. The felt-string in-
(e)
teraction force F i in (6.9) is computed from the 3D hammer felt model with Dirichlet boundary
conditions (6.7) (6.8), and added to the rhs of (2.11) as a non-conservative force. Then, (2.11)
and (6.14) forms 2 sets of coupled equations for 2 sets of variables (u(a) , θ), which provides a
foundation for obtaining a solution theoretically. Despite the linearity of 3D elasticity model,
the felt’s force on the string is still nonlinear because of the interaction judging condition.

6.2    Model for string-soundboard coupling
Piano strings are coupled to the soundboard’s bridge part, which terminates and transmits string
vibration to the soundboard. Figure 6 visualizes this coupling at the bridge from 3 perspectives,
where xyz and x′ y ′ z ′ are the coordinate systems for soundboard and string respectively. Con-
verting a vector (not a point) v in the xyz system into v ′ in the x′ y ′ z ′ system, according to the
angles α and β marked in figure 6, yields v ′ = Rv where R = R1 R2 and
                                                     
cos α 0 sin α                 cos β    sin β 0
R1 =         0    1     0  , R2 =  − sin β cos β 0                       (6.15)
− sin α 0 cos α                  0         0       1

are the rotation matrices. The orthogonal property of rotation matrix yields v = R⊤ v ′ .
We conjecture from observations that the string-soundboard coupling is achieved via several
mechanisms. The first mechanism is “bridge hump coupling”, as illustrated in the side perspective
of figure 6. The middle part of bridge is constructed a bit higher than the agraffe, the starting
point of the speaking string. As a result, the string experiences some upward pressure from the
bridge and the bridge experiences some downward pressure from the string, both statically and
dynamically. The second mechanism is “bridge pin horizontal coupling”, as illustrated in the top
perspective of figure 6. Two bridge pins for one string are drilled into the bridge at positions that
would bend the string a bit horizontally, bringing mutual horizontal tension between the string
and bridge, and restricting the string’s horizontal movement. Made of hard metal like steel,
brass or even titanium, the bridge pins can be treated as rigid bodies with zero strain and stress.
The third mechanism is “bridge pin vertical coupling”, as illustrated in the front perspective of
figure 6. The two bridge pins lean towards their respective string sides, blocking the string from

moving upwards beyond the pins. Also, the notches of bridge ensure that bridge hump coupling
and bridge pin coupling occur at almost the same position, viz. the front bridge pin (the one
closer to the agraffe).
(a)    (a)
On the string’s side, the coupling point is defined as x1 = [L1 , 0, −r(a) ]⊤ , the lowest point
(a)
of string at the front pin. While other relevant points for coupling may be [L1 , −r(a) , 0]⊤ , the
closest point of string to the bridge pin, the string’s displacements and surface forces at these
(a)
points are identical, assuming u(a) does not vary with y (a) or z (a) at x(a) = L1 (no rotation)
(b)
for simplicity6 . The coordinate of this coupling point on the soundboard’s side is denoted x1 .

### 6.2.1 Static coupling
For coupling before motion is initiated, the string and soundboard should have non-zero prestress
fields that attain static balance. It is sufficient to specify that the string prestress only acts
perpendicular to its cross-sections and is constant through its speaking length. This means
in the string’s tension matrix T (a) , T11 is a constant and other entries are zero, immediately
satisfying the static balance. For the soundboard, a full space dependent tension matrix T (b)
with 6 unique elements is necessary. The static balance condition can then be expressed as 3
partial differential equations
∇ · T (b) = 0,                                   (6.16)
which can be solved numerically by FEM using nodal surface forces as time-independent DOFs.
Nonetheless, continuity between string and soundboard tension fields is required. To write the
continuity equations, we define an intermediate coordinate system x′′ y ′′ z ′′ resulting from rotating
the x′ y ′ z ′ system by R⊤    1 or rotating the xyz system by R2 , as marked blue in figure 6. At the
coupling point, the string’s tension on surfaces with outward normals in the y ′′ and z ′′ axes exert
on the bridge, whereas that in the x′′ axis exerts on the farther hitch pins that seem to not
connect the soundboard. This leads to one Dirichlet boundary condition for each string coupled
to the soundboard as
                                                       
(b)                                (a)
R2 T (b) (x1 )R⊤ 2               =   R ⊤ (a)
# 1 T   (x1   )R 1            ,        (6.17)
[22,33,12,13,23]                         [22,33,12,13,23]

where subscript [i1 ,i2 ,...] means taking vector or matrix elements at indices i1 , i2 , ... to form a
vector. This condition can be deduced into 5 equations, leaving 1 DOF for the coupling point.

### 6.2.2 Dynamic coupling
For coupling in motion, the string transmits its surface forces (dynamic prestress, stress and
damping force) to the soundboard. At coupling point, the string’s total surface force, excluding
the static prestress which is already accounted for on the soundboard’s side, is G(a) (x1 , t).
Similar to static coupling, only forces on surfaces with outward normals in the y ′′ and z ′′ axes
are transmitted to the soundboard at the coupling point. This leads to
                           
                    0  Z12 Z13             1
(b)
F (b) (x(b) , t) = δ x(b) − x1 R⊤   2
 Z12 Z22 Z23  R2   1  ,
Z13 Z23 Z33             1
(a)
{Zij } = R⊤
1G
(a)
(x1 , t)R1 ,                                                             (6.18)

where δ(x) is a 3D Dirac delta function indicating a point load. The above equations treats the
string’s surface force as a non-conservative force input to the soundboard system.
Another coupling condition arising from observation is that the string and soundboard should
have the same y ′′ and z ′′ direction displacements7 at the coupling point, written as
                                                     
(a)                           (b)
R⊤1 ∂ t u (a)
(x1   , t)  =   R 2 ∂ t u (b)
(x1   , t)  .        (6.19)
[2,3]                        [2,3]

The reason for this is that the the aforementioned three coupling mechanisms constitute support
for the string’s surfaces with outward normals in the y ′′ and z ′′ axes at the coupling point. This
# 6 This is also true for the string’s central point x(a) = [L(a) , 0, 0]⊤ previously defined.
# 1 1
# 7 Here displacement continuity should be equivalent to velocity continuity, since both the string’s and sound-

board’s displacements are based on a [0, 0, 0]⊤ point.

may not hold for the x′′ axis, but if it is needed8 , the subscript [2,3] in above equations can
be removed to impose a stronger displacement continuity condition. Anyway, restriction of the
string’s x′′ axis (longitudinal) motion is always present at the farther hitch point which may not
connect the soundboard, see (4.3). The same-displacement condition also provides a basis for
establishing the string’s displacement at the coupling point (Dirichlet boundary condition), so
that the string’s PDE (2.11) has an appropriate solution.

### 6.2.3 More discussions on coupling
String-soundboard coupling exhibits complex mechanisms that we find challenging to discover
and describe. Some of these mechanisms that we observe but not covered in the aforementioned
coupling model are discussed in this subsection. These discussions may not cover much details
of computation due to their complexities.
Firstly, we only specified the coupling point at the bridge’s front pin but ignored the rear
pin. As coupling at the rear pin position potentially exists, the string in our model may need to
(a)
be extented in length to cover the segment between the front pin position x1 = [L1 , 0, −r(a) ]⊤
(a)
and the rear pin position x2 = [L2 , 0, −r(a) ]⊤ (we call it “this segment” is this paragraph).
At this segment, the horizontal bending of string may be accouted for in the prestress, but not
necessarily represented in the geometrical volume for simplicity. In static state, the prestress field
coupling should be computed for this segment paying attention that only at the two pin points
are y ′′ axis surface forces coupled. In dynamic state, transmission of the string’s surface forces
should be computed for this segment, paying attention that only at the two pin points are y ′′ axis
surface forces transmitted. A maybe more accurate computation of surface force transmission is,
that at the two pins only when the string’s y ′′ axis surface forces are towards the front or rear
pins should they be transmitted, and that at between (excluding) the two pins only when the
string’s z ′′ axis surface forces are upwards should they be transmitted. This may lead to a highly
nonlinear function akin to the case of hammer-string interaction (6.3), and may make imposing
boundary conditions tougher due to the inequality rather than equality nature. For displacement
coupling at this segment, at the two pins when the string’s y ′′ direction displacements should not
exceed the pins, whereas the z ′′ direction displacements are fully coupled to the pins; at between
(excluding) the string’s z ′′ direction displacements should not be under the soundboard plane,
whereas the y ′′ direction displacements are unrestricted.
Secondly, we considered coupling as occurring at a point, but it may actually occur in a
small contact surface (we call it “this surface” is this paragraph). In such a case, the Dirichlet
boundary condition of prestress field coupling should be specified over this surface; the dynamic
surface force transmission should be treated as surface load rather than point load; the Dirichlet
boundary condition of displacement coupling should also be specified over this surface.
Thirdly, we still lack geometric details regarding the string’s notable vertical and horizontal
bending at the bridge. This bending changes the string’s longitudinal direction, and thereby the
# 3 directions of vibration. Though string vibration should be terminated at the bridge pin, it
seems only at the hitch pin that longitudinal vibration is fully restricted. Therefore, the string’s
segment from front bridge pin to hitch pin may require investigation, which may concern the
duplex scale phenomena [25]. We can design a multi-segment geometric model for the string,
each segment having differently rotated constitutive matrices. Also, the position-dependent static
tension can be specified parallel to the central line of each string segment.
Finally, we ignored the soundboard’s normal and tangent surface forces exerting on the string
at coupling point. This is similar to the case of hammer-string coupling where the string’s surface
forces exerting on the hammer are ignored. This is mainly for practical considerations, as we
would want to solve the soundboard’s equations for only once. Since we did not impose Dirichlet
boundary condition for the soundboard at each coupling point with the around 200 strings,
Neumann boundary conditions apply here restricting the soundboard’s conservative surface forces
to be zero at coupling points. It may be acceptable given the intuitive feeling that “string →
soundboard” transmission should dominate “soundboard → string” transmission. If the latter is
essential, some DOFs may be need to be removed from the coupling point on the soundboard’s
# 8 Maybe in case of high enough viscosity or friction, the string can not slip more than the soundboard in the

x′′ direction. Restricting this movement may also make the solution of string’s PDE more stable. This needs to
be tested.

side, which is a challenge for imposing Dirichlet boundary condition reasonably for both the
string and the soundboard.

6.3      Model for soundboard-air and room-air coupling
Modeling soundboard-air and room-air coupling is crucial for arriving at the final digital audio
sound to the listeners. Generally in the context of solid-fluid coupling dynamics, the interaction
surface should have continuous normal and tangential velocities, as well as continuous normal
and tangential surface forces on both sides [19].
(f )                  (c)
As per section 5, room-air coupling occurs on surface Γ1 (room side) and Γ1 (air side).
(f )           (c)
Define coordinate transformation x = r 1 + R1 x from air coordinates to soundboard coordi-
nates, where r 1 and R1 are the shift vector and rotation matrix. We impose Dirichlet boundary
condition of velocity continuity that
(c)
u(c) (x(c) , t) = R⊤
# 1 u̇
(f )
(x(f ) , t), x(c) ∈ Γ1 , x(f ) = r 1 + R1 x(c) .   (6.20)

We also specify the contribution of air surface force to the room equation’s source term on the
rhs of (2.11) as
(f )                                                   (f )
F 1 (x(f ) , t) = −G(c) (x(c) , t)n(c) (x(c) ), x(f ) ∈ Γ1 , x(c) = −R⊤        ⊤ (f )
# 1 r 1 + R1 x    ,    (6.21)

where n(c) (x(c) ) is the outward normal of the tangent plane of x(c) .
(b)
As per sections 3 and 5, soundboard-air coupling occurs on surface Γ2 (soundboard side)
(c)
and Γ2 (air side). Define coordinate transformation x(b) = r 2 + R2 x(c) from air coordinates to
soundboard coordinates, where r 2 and R2 are the shift vector and rotation matrix. We impose
Dirichlet boundary condition of velocity continuity that
(c)
u(c) (x(c) , t) = R⊤   (b) (b)
# 2 u̇ (x , t), x
(c)
∈ Γ2 , x(b) = r 2 + R2 x(c) .           (6.22)

We also specify the contribution of air surface force to the soundboard equation’s source term
on the rhs of (2.11) as
(b)                                                   (b)
F 1 (x(b) , t) = −G(c) (x(c) , t)n(c) (x(c) ), x(b) ∈ Γ2 , x(c) = −R⊤        ⊤ (b)
# 2 r 2 + R2 x ,         (6.23)

where n(c) (x(c) ) is the outward normal of the tangent plane of x(c) .

# 7 Numeric schemes
7.1      Modal superposition method for solving coupled ODEs
### 7.1.1 Modal transformation of second-order ODEs
For solving coupled second-order ODEs like (2.19), time discretization using finite-difference is a
direct approach. However, given that damping matrix is diagonalizable by the mass and stiffness
matrices, decoupling and dimension reduction of the system by means of modal superposition is
preferred. To do so, we first define the following eigenvalue problem

Kϕi = λi M ϕi ⇐⇒ M −1 Kϕi = λi ϕi (i = 1, ..., N ),                        (7.1)

and the eigen decomposition

K = M ΦΛΦ−1 ⇐⇒ M −1 K = ΦΛΦ−1 ,                                     (7.2)

where λi is a (maybe complex) eigenvalue, ϕi is a N × 1 (maybe complex) eigenvector, Λ =
diag(λ1 , ..., λN ) is a diagonal matrix of eigenvalues, Φ = [ϕ1 , ..., ϕN ] is a N × N matrix of
eigenvectors. Note that since both M and K are sparse matrices with large dimensions, it is
preferrable to solve the generalized eigenvalue problem on the left side of (7.1), avoiding explicit
inversion of M that would otherwise result in a large dense matrix. We can leverage existing
softwares of sparse eigensolvers like Arpack and FEAST to solve the generalized eigenvalue
problem. Nevertheless, for solving ODEs of the acoustic system with large DOFs, it may be

unwise to store all rows of eigenvectors at the same time which may otherwise lead to memory
overload. A re-implementation of existing sparse eigensolver algorithms may be desired so as to
store partial rows or columns of eigenvectors in a “rolling” way.
Since eigenvectors are linearly independent, that is to say Φ is invertible, we can write the
solution in the form ξ(t) = Φq(t) where q(t) is an N × 1 vector we call as modal DOFs. Note
here ξ(t) should be real (in the complex domain), but q(t) may be complex because Φ−1 may
be complex. Then (2.19) can be decoupled as
M Φq̈(t) + 2µKΦq̇(t) + KΦq(t) = f (t)                     (7.3a)
H                       H                  H                   H
Φ M Φq̈(t) + 2µΦ KΦq̇(t) + Φ KΦq(t) = Φ f (t)                         (7.3b)
H
M̄ q̈(t) + 2µM̄ Λq̇(t) + M̄ Λq(t) = Φ f (t)             (7.3c)
q̈(t) + 2µΛq̇(t) + Λq(t) = p(t)             (7.3d)
where
−1
M̄ = ΦH M Φ, p(t) = M̄                 ΦH f (t);            (7.4)
are called the modal mass matrix and the modal force vector respectively. Through eigen de-
composition, the N coupled ODEs have now been transformed into N uncoupled ODEs. Note
that in case the damping matrix can not be diagonalized by the mass and stiffness matrices, we
should rewrite the second-order ODEs as first-order ODEs with doubled number of DOFs, so
that it can be decoupled as will be shown in section 7.1.2.
It is well-understood that the analytical solution of the i th uncoupled second-order ODE
without the rhs nonhomogenous term is a sinusoidal signal with a single eigenfrequency positively
correlated to the magnitude of i th eigenvalue. Assuming eigenvalues in Λ are ascendingly sorted
by their real magitudes, we can select only the lowest M eigenvalues and discard the rest, because
most human ears are insensitive to eigenfrequencies above a certain threshold (often 10 kHz).
This is also for practical considerations that the number of DOFs is often too large for numeric
computation, making dimension reduction desirable. Consequently, we obtain a M × 1 vector
q ′ (t) with only the first M entries of q(t), and the M + 1 to N entries are all approximated as
zeros and discarded. Subsituting q(t) by q ′ (t) (more exactly, q ′ (t) should be padded N − M
zeros at the end) in (7.3c), and omitting the last N − M equations and unknowns, yields
S ′ q̈ ′ (t) + 2µS ′ Λ′ q̇ ′ (t) + S ′ Λ′ q ′ (t) = Φ′⊤ f (t)   (7.5a)
q̈ ′ (t) + 2µΛ′ q̇ ′ (t) + Λ′ q ′ (t) = p′ (t)      (7.5b)
where Λ′ has dimension M × M containing the first M eigenvalues and Φ′ has dimension N × M
containing the first M eigenvectors. The large numbe of DOFs is now approximated as the linear
combination of a smaller number of modal DOFs as ξ(t) ≈ Φ′ q ′ (t). The reduced modal mass
and modal force are
′                        ′
M̄ = Φ′H M Φ′ , p(t) = M̄ −1 Φ′H f (t).                     (7.6)
For around 2500 modes of the soundboard system as estimated in [10], the fully dense modal
mass matrix (128 bit complex type) would require about 95 MB memory which is normally
acceptable in both storage and computation aspects. But this dense matrix can be avoided if
the mass and stiffness matrices are both symmetric because in this case the eigenvectors can be
normalized so that the modal mass matrix equals to an identity matrix, making its inversion much
easier. This benefit is achievable in our 3D elastic solid model because the constitutive matrix
is symmetric even in the presence of prestres. A symmetric stiffness matrix often means that
energy is conserved within the system if there is no other non-conservative forces like damping
force.
Now we consider the analytical solution of the i th uncoupled nonhomogenous ODE
q̈(t) + 2µλq̇(t) + λq(t) = p(t),                        (7.7)
where the subscripts i and superscripts ′ are dropped for convenience. Applying Laplace trans-
form to this equation yields
s2 Q(s) + 2µλsQ(s) + λQ(s) = P (s) + Q0 (s),
s2 G(s) + 2µλsG(s) + λG(s) − G0 (s) = 1
g̈(t) + 2µλġ(t) + λg(t) = δ(t)                      (7.8)

where s is a complex variable, δ(t) is the Dirac delta function; the initial parts of Laplace
transforming derivatives are defined as

Q0 (s) = sq(0) + q̇(0) + 2µλq(0)
G0 (s) = sg(0) + ġ(0) + 2µλg(0);                          (7.9)

the well-known Green’s function (frequency domain), the response to a unit impulse, is defined
as
Q(s) [1 + G0 (s)]          G(s) [P (s) + Q0 (s)]
G(s) =                    ⇒ Q(s) =                       .            (7.10)
P (s) + Q0 (s)                1 + G0 (s)
To solve g(t), we first specify that response should not exist during zero and negative time, viz.
g(t) = ġ(t) = 0 for t ≤ 0; then notice when t > 0, (7.8) becomes homogenous with solution
p
g(t) = C1 exp(z1 t) + C2 exp(z2 t), z1 , z2 = −µλ ± (µ2 λ − 1)λ.

To determine complex constants C1 , C2 , we first notice that g(t) should be continuous at t = 0
in the presence of second derivative in (7.8), thus

g(0+ ) = g(0) ⇒ C1 + C2 = 0.                            (7.11)

Then, integrating (7.8) over (−∞, +∞) yields
Z +∞                                      Z +∞
[g̈(t) + 2µλġ(t) + λg(t)] dt =            δ(t)dt
−∞                                         −∞
Z 0+
⇒          g̈(t)dt = ġ(0+ ) − ġ(0− ) = ġ(0+ ) = 1
0−
⇒C1 z1 + C2 z2 = 1,                                                  (7.12)

which is the so-called jump discontinuity condition for first derivative; the continuity condition
for the zero-order has been applied again here. From (7.11) (7.12) the complex constants are
found to be
# 1 1
C1 =          , C2 =          .                            (7.13)
z1 − z2         z2 − z1
Subsituting g(0) = 0 and ġ(0) = 0 into (7.9) yields G0 (s) = 0. Then from (7.10) we have

Q(s) = P (s)G(s) + [q̇(0) + 2µλq(0)] G(s) + sq(0)G(s)
q(t) = p(t) ∗ g(t) + [q̇(0) + 2µλq(0)] g(t) + q(0)ġ(t),              (7.14)

where ∗ is the convolution operator over [0, +∞). For the general case of initial conditions
q(0) = q̇(0) = 0, the solution reduces to q(t) = p(t)∗g(t). It is now clear that the nonhomogenous
ODEs can be solved by convolving the modal force p(t) (source signal) with the Green’s function
g(t) (response signal), which can be efficiently computed using fast Fourier transform (FFT)
convolution in the frequency domain. However, the analytical solution may not be actually useful
or efficient in the presence of coupling between systems and the explicit time discretization scheme
introduced in section 7.2 will be an alternative. Nevertheless, it provides an understanding of
the characteristics of vibration modes.

### 7.1.2 Modal transformation of first-order ODEs
Solving coupled first-order ODEs like (5.14) by means of modal superposition is similar to the
second-order case, and we only discuss some particularities here. Firstly, the eigenvalue problem
is defined in the same way for M (now for first-order) and K. The decoupled ODEs write

M̄ q̇(t) + M̄ Λq(t) = ΦH f (t)                        (7.15a)
q̇(t) + Λq(t) = p(t)                            (7.15b)

where
−1
M̄ = ΦH M Φ, p(t) = M̄            ΦH f (t)                 (7.16)

are the modal mass and the modal force. Specific attension should be paid to the analytical
solution of the i th uncoupled nonhomogenous ODE

q̇(t) + λq(t) = p(t).                               (7.17)

Laplace transform yields

sQ(s) + λQ(s) = P (s) + q(0),
sG(s) + λG(s) − g(0) = 1
ġ(t) + λg(t) = δ(t)                               (7.18)

where the Green’s function is defined as
Q(s) [1 + g(0)]          G(s) [P (s) + q(0)]
G(s) =                   ⇒ Q(s) =                     .               (7.19)
P (s) + q(0)                 1 + g(0)

Note that g(t) = ġ(t) = 0 for t ≤ 0 is still required, but continuity g(0) = g(0+ ) is unnecessary
R
because the highest order of derivative in (7.18) is only one. Nevertheless, continuity of g(t)dt
at t = 0 can be easily satisfied because the primitive function can have an arbitrary constant
added. Then integrating (7.18), we can find g(0+ ) = 1 and the solution of Green’s function
g(t) = − exp(−λt).                                   (7.20)
λ
It follows that

Q(s) = P (s)G(s) + q(0)G(s)
q(t) = p(t) ∗ g(t) + q(0)g(t),                          (7.21)

is the solution of the uncoupled nonhomogenous first-order ODE.

7.2    Explicit time discretization for coupling between systems
Having discussed the weak form ODEs for each subsystem of the physical piano, our question
is then how a numerical treatment of coupling between system may be achieved that attains
a sensible tradeoff between feasibility and accuracy. For most subsystems presented in the
previous sections, there exists an external force term on the rhs of PDEs. This, along with the
predetermined displacements on the boundary, form the main contributions to the rhs source
term of system ODEs.
Nevertheless, to say “ODEs” here is actually indefensible, because in many coupling cases
discussed before the rhs source term f (t) depends linearly or nonlinearly on the lhs unknown
DOFs ξ(t) too. In such cases, eigen decomposition of “pseudo ODEs” can only obtain decoupled
lhs operators on q(t) on the lhs, but rhs operators on q(t) often remain coupled, not only
because of the nonlinearity of operators, but also because eigen decomposition applies to the
local subsystem but not the global system where rhs sources come from. Hammer-string coupling
is a typical example: the string displacement depends on the felt’s force, the felt’s force depends
on its compression, but this compression depends on string displacement. Another example
is in soundboard-string coupling, the soundboard displacement at the coupling point depends
on the string’s surface force, the string’s surface force depends on its displacement, computing
its displacement requires knowing the coupling point displacement (Dirichlet boundary), but
this displacement depends on the soundboard displacement at the coupling point to satisfy the
continuity condition.
An explanation for these seemingly odd relations is that a theoretically sound way would seem
to do computations as if all subsystems were a single system, rather than to separate systems
and reintroduce coupling between them. This means all DOFs are integrated into a single vector
of DOFs, all mass (or stiffness) matrices are assembled into a bigger one, displacements at
coupling positions are expressed by the same unique DOFs behind, in order for full coupling
solutions. However, this is often unacceptable not only because of the high costs of storage and
computation, but also because the inherent heterogeneities of different subsystems, particularly
the nonlinearity of hammer felt compression, the first-order characteristic of acoustic system, the

different damping mechanisms, may actually not cohere well and may even be tough to control
in a single system. Therefore, we shall still adopt the framework of separate systems with mutual
coupling, and seek for numeric schemes capable of achieving a level of accuracy as close to full
coupling schemes as possible.
The time domain scheme we shall introduce here is inspired by the idea of velocity Verlet
algorithms, but customized for our case of modal-transformed first and second order ODEs,
with second-order accuracy9 . Despite being lower-order compared to the complex time schemes
in [14, 8], our scheme may be more efficient particularly in eliminating the need to invert or
solve big or dense matrices at each step, even when the mass matrix is non-diagonal. Due to the
existence of coupling, the analytical solutions of first and second order ODEs discussed in section
7.1 may be not applicable here. Nevertheless, system decoupling and dimension reduction via
modal decomposition will be shown useful in improving the efficiency of time-stepping algorithms.

### 7.2.1 Time stepping of second-order ODEs
In the previously derived second-order decoupled ODEs (7.3d), the rhs source term can rewritten
as
q̈(t) + 2µΛq̇(t) + Λq(t) = p(t), p(t) = r (t, q(t), q̇(t)) ,        (7.22)
where r incoporate all DOF-independent and DOF-dependent contributions to the source term;
DOF-dependent contributions consist of two major sources: non-zero Dirichlet boundary condi-
tions; non-conservative force (but excluding damping force already on the lhs).
To perform time discretization, we define the discrete time interval as h = ∆t, which can
be chosen as 1/44100 seconds for producing 44.1 kHz digital audio; define the n th discrete
point in time as tn = nh. The notion of discrete points in time form the basis of time stepping
algorithms, where integrations are performed between time steps. An integral over a small
interval [x0 , x0 + ∆x] can be approximated as area of trapeziud
Z x0 +h
h
f (x)dx ≈     [f (x0 ) + f (x0 + h)] ,                    (7.23)
x0                    2

which can achieve a relatively high accuracy as per the mean value theorem for definite integrals,
though f (x0 + h) may not be known beforehand in some contexts. If we seek for a first-order
approximation of f (x0 + h), then
Z x0 +h
h2 ′
f (x)dx ≈ hf (x0 ) +        f (x0 )                      (7.24)
x0                              2

is a less accurate approximation. Utilizing these integration strategies, we integrate (7.22) over
[tn , tn+1 ] to get
Z tn+1              Z tn+1
q̇(tn+1 ) − q̇(tn ) = −2µΛ [q(tn+1 ) − q(tn )] − sΛ                 q(t)dt +            p(t)dt
tn                  tn
h                                  h2
q̇(tn+1 ) − q̇(tn ) ≈ −2µΛ [q(tn+1 ) − q(tn )] −        Λ [q(tn ) + q(tn+1 )] + hp(tn ) + ṗ(tn ). (7.25)
# 2 2
Also it is obvious that
Z tn+1
h
q(tn+1 ) − q(tn ) =             q̇(t)dt ≈     [q̇(tn ) + q̇(tn+1 )] .         (7.26)
tn                 2

The solution of (7.25) (7.26) is

h2
                                                 
q(tn+1 ) = Z −1
# 1 Z 0 q(t n ) + 2 q̇(t n ) + hp(t n ) +    ṗ(t n )  ,
# 2 2
q̇(tn+1 ) = q(tn+1 ) − q(tn ) − q̇(tn ),                                       (7.27)
h             h
# 9 Alternatively, one may consider transforming time domain into frequency domain to explore coupling from

the perspective of mobility [34].

where
       
# 2 h
Z0 =   I + 2µ −     Λ,
h          2
       
# 2 h
Z 1 = I + 2µ +      Λ.                                  (7.28)
h          2

are diagonal matrices easy to invert. (7.27) reveal that q(tn+1 ), q̇(tn+1 ) can be computed with
second-order accuracy using q(tn ), q̇(tn ), p(tn ), ṗ(tn ), but without using unknowns of the tn+1
step. This means our time discretization scheme is explicit. As for its energy property, readers
can go to appendix D for a detailed deduction incorporating nonlinear strain energy. From the
deduction there, our preliminary judgement is that this explicit scheme is energy stable at least
for the kinetic energy, potential energy and damping, but uncertain for other non-conservative
forces on the rhs. The most significant accuracy loss of this scheme seems to be the use of
(7.24) rather than (7.23) in approximating the integral of p(t), which may also lead to energy
unstability. We would expect the first-order approximation of p(tn+1 ) to be acceptable with
small h. This actually implies that we rely on some historical (tn ) displacement and velocity
values to predict the current (tn+1 ) input sources, then the current input sources are updated
by the computed current displacement and velocity values. This process can be repeated several
times for one time step if higher accuracy and better energy stability is desired, able to use (7.23)
rather than (7.24) in approximating the integral of p(t).

### 7.2.2 Time stepping of first-order ODEs
Recall the previously derived first-order decoupled ODEs (7.15b), the rhs source term can rewrit-
ten as
q̇(t) + Λq(t) = p(t), p(t) = r (t, q(t)) ,                    (7.29)
where r incoporate all DOF-independent and DOF-dependent contributions to the source term;
DOF-dependent contributions consist of two major sources: non-zero Dirichlet boundary con-
ditions; non-conservative force (but excluding damping force already on the lhs). Integrating
(7.29) over [tn , tn+1 ], the approximate solution of q(tn+1 ) can be found as
Z tn+1          Z tn+1
q(tn+1 ) − q(tn ) = −Λ           q(t)dt +        p(t)dt
tn               tn
h                                h2
q(tn+1 ) − q(tn ) ≈ − Λ [q(tn ) + q(tn+1 )] + hp(tn ) + ṗ(tn )
# 2 2
−1 
h2
                                              
h                h
q(tn+1 ) = I + Λ           I − Λ q(tn ) + hp(tn ) + ṗ(tn ) .                (7.30)
# 2 2                      2

Therefore, q(tn+1 ) can be computed using q(tn ), p(tn ), ṗ(tn ), but without using unknowns of
the tn+1 step.

### 7.2.3 Time stepping of hammer shank rotation ODE
The equation of hammer shank motion (6.14) can be written in a more abstract way as

I θ̈ = −µθ̇ + T (θ),                                 (7.31)

where T (θ) incoporate the contributions of gravity and felt force to the total torque of shank,
dependent on the zero-order value of θ. Similar to second-order system ODEs, the time dis-
cretization of (7.31) is found to be
h                  i                           Z tn+1
I θ̇(tn+1 ) − θ̇(tn ) = −µ [θ(tn+1 ) − θ(tn )] +        T (θ)dt
tn
h2
                               
# 2 2
I     θ(tn+1 ) − θ(tn ) − 2θ̇(tn ) ≈ −µ [θ(tn+1 ) − θ(tn )] + hT (tn ) + Ṫ (tn )
h             h                                                      2
          −1                                         2

# 2 2                                     h
θ(tn+1 ) =       I +µ           I + µ θ(tn ) + 2θ̇(tn ) + hT (tn ) + Ṫ (tn ) ,   (7.32)
h              h                                     2

and
# 2 2
θ̇(tn+1 ) =     θ(tn+1 ) − θ(tn ) − θ̇(tn ).                   (7.33)
h           h
Therefore, θ(tn+1 ) and θ̇(tn+1 ) can be computed using θ(tn ), θ̇(tn ), T (tn ), Ṫ (tn ), but without
using unknowns of the tn+1 step.

7.3     Concatenating the whole model
With modal transformation and time discretization of the ODEs established, it now suffices to
combine all parts of the piano model together. The whole computation process consists of two
major separate parts: first do space discretization, then do time discretization. Rooted in the
time-space separation idea of FEM, these two numeric works do not interfere so computation
costs should be acceptable.

### 7.3.1 Space discretization
Derive ODEs for each subsystem.

M ξ̈(t) + C ξ̇(t) + Kξ(t) = f (t), superscripts : a, b, e, f
M ξ̇(t) + Kξ(t) = f (t), superscripts : c
θ̈ = −µθ̇ + T (θ), superscripts : d

Solve generalized eigenvalue problems. Only find the lowest M eigenvalues and corre-
sponding eigenvectors, and integrate them into Λ, Φ.

Kϕi = λi M ϕi , i = 1, ..., M, superscripts : a, b, c, e, f

Compute modal force transformation matrix (dense). This is useful for transformation
p(t) = Sf (t).
      −1
S = ΦH M Φ     ΦH , superscripts : a, b, c, e, f

### 7.3.2 Time discretization
Update schemes for modal DOFs Below lists the previously introduced time discretization
shemes to be later referred to. For each time step n, schemes 1a/2a/3a are preferred over
1b/2b/3b whenever possible, because they utilize values of the rhs source term for next time
step n + 1.

• Update scheme 1a:
     −1                                  
h           h          h        h
q(tn+1 ) = I + Λ       I − Λ q(tn ) + p(tn ) + p(tn+1 )
# 2 2          2        2

• Update scheme 1b:
−1 
h2
                                                 
h            h
q(tn+1 ) =       I+     Λ       I − Λ q(tn ) + hp(tn ) + ṗ(tn )
# 2 2                    2

• Update scheme 2a:
                                          
h        h
q(tn+1 ) = Z −1
# 1 Z 0 q(tn ) + 2q̇(tn ) + p(tn ) + p(tn+1 )
# 2 2
# 2 2
q̇(tn+1 ) = q(tn+1 ) − q(tn ) − q̇(tn )
h           h

• Update scheme 2b:

h2
                                      
q(tn+1 ) = Z −1
# 1 Z 0 q(tn ) + 2q̇(tn ) + hp(tn ) + ṗ(tn )
# 2 2
q̇(tn+1 ) = q(tn+1 ) − q(tn ) − q̇(tn )
h           h

• Update scheme 3a:
−1 
                                               
# 2 2                          h         h
θ(tn+1 ) =     I +µ          I + µ θ(tn ) + 2θ̇(tn ) + T (tn ) + T (tn+1 )
h             h                          2         2
# 2 2
θ̇(tn+1 ) = θ(tn+1 ) − θ(tn ) − θ̇(tn )
h          h

• Update scheme 3b:
−1 
h2
                                                 
# 2 2
θ(tn+1 ) =     I +µ          I + µ θ(tn ) + 2θ̇(tn ) + hT (tn ) + Ṫ (tn )
h             h                                     2
# 2 2
θ̇(tn+1 ) = θ(tn+1 ) − θ(tn ) − θ̇(tn )
h          h

Initialization

1. Discrete time interval h, number of time steps n1 .
′        ′
2. Hammer shank: initial angle and angular velocity θ(d ) (t0 ), θ̇(d ) (t0 ), from which the initial
′                                ′
torque T (d ) (t0 ) and its derivative Ṫ (d ) (t0 ) (gravity contribution only) can be computed.
3. Hammer felt & string & soundboard & air & room barraiers: initial modal DOFs and
modal forces

q (∗) (t0 ) = q̇ (∗) (t0 ) = p(∗) (t0 ) = ṗ(∗) (t0 ) = 0, ∗ = a, b, c, e′′ , f

At each time step n = 0, 1, 2, ..., n1

1. Compute the next rotation of hammer shank using scheme 3b:
′              ′               ′         ′                   ′        ′
θ(d ) (tn ), θ̇(d ) (tn ), T (d ) (tn ), Ṫ (d ) (tn ) → θ(d ) (tn+1 ), θ̇(d ) (tn+1 )

2. Compute the current modal forces of hammer felt (non-zero boundary condition) using
(6.7):
′             ′                                        ′′            ′′
θ(d ) (tn ), θ̇(d ) (tn ), q (a) (tn ), q̇ (a) (tn ) → p(e ) (tn ), ṗ(e ) (tn )
′′
Note: p(e ) (tn ) relates to the displacement of felt at the contact points with string. The
algorithm in appendix C may be more accurate for the hammer felt, but for simplicity we
choose to not integrate it here.

3. Compute the next modal DOFs of hammer felt using scheme 2b:
′′                 ′′           ′′            ′′                  ′′           ′′
q (e ) (tn ), q̇ (e ) (tn ), p(e ) (tn ), ṗ(e ) (tn ) → q (e ) (tn+1 ), q̇ (e ) (tn+1 )

4. Compute the next torque of hammer shank using (6.14):
′                     ′               ′′                  ′′                  ′            ′
θ(d ) (tn+1 ), θ̇(d ) (tn+1 ), q (e ) (tn+1 ), q̇ (e ) (tn+1 ) → T (d ) (tn+1 ), Ṫ (d ) (tn+1 )

5. Compute the next modal forces of string (non-conservative force) using (6.9):
′′                      (a)
q (e ) (tn+1 ) → p1 (tn+1 )
(a)                                                     (a)
Note: p1         relates to hammer felt force, p2                     relates to bridge point displacement.

(a)                               (a)
6. Compute the next modal DOFs of string using scheme 2a for p1                                    and scheme 2b for p2 :
(a)          (a)             (a)             (a)
q (a) (tn ), q̇ (a) (tn ), p1 (tn ), p1 (tn+1 ), p2 (tn ), ṗ2 (tn ) → q (a) (tn+1 ), q̇ (a) (tn+1 )

7. Compute the next modal forces of soundboard (non-conservative force) using (6.18):
(b)
q (a) (tn+1 ) → p1 (tn+1 )
(b)                                                       (b)
Note: p1 relates to string force at the bridge, p2 relates to air pressure on the soundboard.
(b)
8. Compute the next modal DOFs of soundboard using scheme 2a for p1 and scheme 2b for
(b)
p2 :
(b)         (b)             (b)             (b)
q (b) (tn ), q̇ (b) (tn ), p1 (tn ), p1 (tn+1 ), p2 (tn ), ṗ2 (tn ) → q (b) (tn+1 ), q̇ (b) (tn+1 )

9. Compute the next modal forces of air (non-zero boundary condition) using (6.22):
(c)
q (b) (tn+1 ) → p1 (tn+1 )
(c)                                                                                            (c)
Note: p1 relates to displacements at the interface with soundboard, p2                                        relates to dis-
placements at interface with room barriers.
(c)                               (c)
10. Compute the next modal DOFs of air using scheme 1a for p1 and scheme 1b for p2 :
(c)         (c)               (c)          (c)
q (c) (tn ), p1 (tn ), p1 (tn+1 ), p2 (tn ), ṗ2 (tn ) → q (c) (tn+1 )

11. Compute the next modal forces of soundboard (non-conservative force) using (6.23):
(b)               (b)
q (c) (tn+1 ) → p2 (tn+1 ), ṗ2 (tn+1 )

12. Compute the next modal forces of room barriers (non-conservative force) using (6.21):

q (c) (tn+1 ) → p(f ) (tn+1 )
′′
Note: p(e ) (tn ) relates to air pressure on the room barriers.

13. Compute the next modal DOFs of room barriers using scheme 2a:

q (f ) (tn ), q̇ (f ) (tn ), p(f ) (tn ), p(f ) (tn+1 ) → q (f ) (tn+1 ), q̇ (f ) (tn+1 )

14. Compute the next modal forces of air (non-zero boundary condition) using (6.20):
(c)          (c)
q (f ) (tn+1 ), q̇ (f ) (tn+1 ) → p2 (tn+1 ), ṗ2 (tn+1 )

15. If higher accuracy and better energy stability is desired, repeat the above steps 1 to 14
several times. Note that different from the first iteration, the subsequent iterations always
use schemes 1a, 2a, 3a and do not need to use schemes 1b, 2b, 3b. If no more repetition is
needed, end the current time step and move on to the next time step.

Audio output Compute the acoustic pressure signals at certaining listening positions using
the stored modal DOFs q (c) (tn ) (n = 0, ..., n1 ) and modal superposition. These signals are the
final output digital audio of piano simulation model.

# 8 Conclusion
This paper presented a detailed physical model for simulating acoustic piano sounds. For solid
parts of the piano system, viz. strings, soundboard, room barriers, hammer felt, a 3D prestressed
elasticity model is generally applied. For fluid parts of the piano system, viz. sound radiation in
the air, conservation of mass and Navier-Stokes equation is applied. For coupling between differ-
ent subsystems of the piano, mechanisms of surface force transmission and displacement/velocity
continuity are considered. For numeric simulation, modal superposition and explicit time dis-
cretization schemes are utilized. Despite the complexity of this whole piano model, we have paid
efforts to a straightforward presentation based more on system ODEs transformed from strong
PDEs, as well as a time domain simulation scheme balancing efficiency and accuracy.
Below discusses the current study’s limitations and our plans or recommendations for future
research.

• Waiting for numeric simulation results. Due to the complication of our piano model, we
choose to write down theoretic models first as a guiding framework. Our next step involves
implementing the computation procedures using high-performance, expressive and well-
structured programming languages like Rust, as well as performing result analysis using
convenient and ecologically rich programming languages like Python. Facing some unknown
uncertainties in practice, our model needs to be further tested and improved.
• In hammer felt-string coupling and string-soundboard coupling, the contact was treated
as occuring at a point rather than an area. This simplification would probably lead to a
loss in realism. The 3D contact framework described in appendix C may apply here, but
algorithmic complexity and numeric stability needs to be better addressed.

• The string-soundboard coupling mechanism was assumed of fixation but not collision na-
ture. A coupling model similar to the nonlinear hammer-string interaction may be more
suitable, considering that a string seems to actually be supported between two distanced
bridge pins. The 3D contact framework described in appendix C may apply here, but
algorithmic complexity and numeric stability needs to be better addressed. It also remains
to discover how the relative positions of the two pins affect the vibrations of the string and
soundboard.
• Treatment of nonlinear coupling between the distinct subsystems of the piano in time
discretization schemes remains challenging. As far as we understand, only nonlinear implicit
schemes can achieve unconditional energy stability if the two coupling mechanisms we
consider, namely the surface force transmission and the displacement/velocity continuity,
exhibit nonlinearity. A viable tradeoff between accuracy and efficiency may be the explicit
iteration approach we proposed, but we hope to see better alternatives.
• The damping in our model is relatively simplified, as only one or two viscousity coefficients
are used. It may be beneficial to introduce additional unknowns like temperature and
additional governing equations like thermal equations to reflect the damping phenomena
more realistically.
• Some observed phenomena of acoustic pianos are still missing their representations in our
model. For example, strings not striked by the hammer may also vibrate as long as the
sustain pedal is pressed, which is often called sympathetic resonance. An explanation for
this is that the striked string transmits its vibration to other strings through the variation of
air pressure, which can actually lead to a string-air coupling model. Also, the transmission
of piano player’s key action into hammer shank movement requires further investigation.
• It remains to discover the relation between physical parameters of the piano model and the
objective (waveforms, spectrums) and subjective (listener feel) aspects of the final ouput
sound, so that these parameters can be tuned to approach realism or even obtain new
sounds without a real world couterpart.
Finally, we hope this study could contribute to the understanding of the vibroacoustics of a
piano, and to the innovation of digital musical instruments with desired characteristics of sound.

List of Figures
# 1 The 9 surface forces acting on the 3 positive sides of an infinitesimal volume . . .       3
# 2 The piano soundboard in top view (left) and side view (right) . . . . . . . . . . .        7
# 3 The piano string in 3D view . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .    8
# 4 The piano in a room . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .    9
# 5 The piano hammer view from y (left) and x (right) direction . . . . . . . . . . .         13
# 6 The piano bridge view from 3 directions . . . . . . . . . . . . . . . . . . . . . . .     17

References
[1] Constitutive equations.    http://web.mit.edu/16.20/homepage/3_Constitutive/
Constitutive_files/module_3_no_solutions.pdf. Accessed: 2024-08-06.

[2] Stress and equations of motion.          https://www.comsol.com/multiphysics/
stress-and-equations-of-motion. Accessed: 2024-09-17.
[3] Rolf Bader and Niko Plath. Impact of damping on oscillation patterns on the plain piano
soundboard. In Acoustics, volume 4, pages 1013–1027. MDPI, 2022.

[4] Balazs Bank and Juliette Chabassier. Model-based digital pianos: from physics to sound
synthesis. IEEE Signal Processing Magazine, 36(1):103–114, 2018.
[5] Balázs Bank and László Sujbert. Generation of longitudinal vibrations in piano strings:
From physics to sound synthesis. The Journal of the Acoustical Society of America,
117(4):2268–2278, 2005.

[6] Abhiram Bhanuprakash, Maarten Van Walstijn, and Vasileios Chatziioannou. Quadratic
spline approximation of the contact potential for real-time simulation of lumped collisions
in musical instruments. In 27th International Conference on Digital Audio Effects (DAFx24),
pages 155–162, 2024.

[7] Voichita Bucur. The Acoustics of Wood (1995). CRC press, 2017.
[8] Guillaume Castera and Juliette Chabassier. Numerical analysis of quadratized schemes.
Application to the simulation of the nonlinear piano string. PhD thesis, Inria, 2023.
[9] Guillaume Castera, Juliette Chabassier, Paul Fisette, and Brad Wagijo. Piano bridge mo-
bility and longitudinal precursors. PhD thesis, Inria Bordeaux-Sud Ouest, 2023.

[10] Juliette Chabassier, Antoine Chaigne, and Patrick Joly. Modeling and simulation of a grand
piano. The Journal of the Acoustical Society of America, 134(1):648–665, 2013.
[11] Juliette Chabassier, Antoine Chaigne, and Patrick Joly. Time domain simulation of a pi-
ano. part 1: model description. ESAIM: Mathematical Modelling and Numerical Analysis,
48(5):1241–1278, 2014.
[12] Juliette Chabassier and Marc Duruflé. Energy based simulation of a timoshenko beam in
non-forced rotation. influence of the piano hammer shank flexibility on the sound. Journal
of Sound and Vibration, 333(26):7198–7215, 2014.
[13] Juliette Chabassier, Marc Duruflé, and Patrick Joly. Time domain simulation of a piano. part
2: numerical aspects. ESAIM: Mathematical Modelling and Numerical Analysis, 50(1):93–
133, 2016.
[14] Juliette Chabassier and Sébastien Imperiale. Introduction and study of fourth order theta
schemes for linear wave equations. Journal of Computational and Applied Mathematics,
245:194–212, 2013.

[15] Antoine Chaigne and Jean Kergomard. Acoustics of musical instruments. Springer, 2016.
[16] Michele Ducceschi and Stefan Bilbao. Simulation of the geometrically exact nonlinear string
via energy quadratisation. Journal of Sound and Vibration, 534:117021, 2022.

[17] Michele Ducceschi, Stefan Bilbao, Craig J Webb, et al. Real-time simulation of the struck
piano string with geometrically exact nonlinearity via a scalar quadratic energy method. In
Proceedings of the ENOC 2022-10th European Nonlinear Dynamics Conference, pages 1–8,
2022.
[18] Michele Ducceschi, Stefan Bilbao, Silvin Willemsen, and Stefania Serafin. Linearly-implicit
schemes for collisions in musical acoustics based on energy quadratisation. The Journal of
the Acoustical Society of America, 149(5):3502–3516, 2021.
[19] F Dunn, WM Hartmann, DM Campbell, and Neville H Fletcher. Springer handbook of
acoustics. Springer, 2015.
[20] Benjamin Elie, Benjamin Cotté, and Xavier Boutillon. Physically-based sound synthesis
software for computer-aided-design of piano soundboards. Acta Acustica, 6:30, 2022.
[21] Brian Hamilton and Stefan Bilbao. Fdtd modelling of sound propagation in air including
viscothermal and relaxation effects. In Forum Acusticum, pages 541–543, 2020.
[22] Brian Hamilton and Stefan Bilbao. Time-domain modeling of wave-based room acoustics
including viscothermal and relaxation effects in air. JASA Express Letters, 1(9), 2021.
[23] Mario Igrec. Pianos inside out, 2013.
[24] Nam-Ho Kim and Nam-Ho Kim. Finite element analysis for contact problems. Introduction
to nonlinear finite element analysis, pages 367–426, 2015.
[25] Pablo Miranda Valiente, Giacomo Squicciarini, and David J Thompson. Influence of sound-
board modelling approaches on piano string vibration. The Journal of the Acoustical Society
of America, 155(5):3213–3232, 2024.
[26] Philip McCord Morse and K Uno Ingard. Theoretical acoustics. Princeton university press,
1986.
[27] Alan T Nettles. Basic mechanics of laminated composite plates. Technical report, 1994.
[28] Annamaria Pau and Francesco Lanza di Scalea. Nonlinear guided wave propagation in
prestressed plates. The Journal of the Acoustical Society of America, 137(3):1529–1540,
2015.
[29] Jie Shen, Jie Xu, and Jiang Yang. The scalar auxiliary variable (sav) approach for gradient
flows. Journal of Computational Physics, 353:407–416, 2018.
[30] Jie Shen, Jie Xu, and Jiang Yang. A new class of efficient and robust energy stable schemes
for gradient flows. SIAM Review, 61(3):474–506, 2019.
[31] Jin Jack Tan. Piano acoustics: string’s double polarisation and piano source identification.
PhD thesis, Université Paris Saclay (COmUE), 2017.
[32] Jin-Jack Tan, Cyril Touzé, and Benjamin Cotté. Double polarisation in nonlinear vibrating
piano strings. In Third Vienna Talk on Music Acoustics, 2015, 2015.
[33] Benjamin Trévisan, Kerem Ege, and Bernard Laulagnet. A modal approach to piano sound-
board vibroacoustic behavior. The Journal of the Acoustical Society of America, 141(2):690–
709, 2017.
[34] Pablo Miranda Valiente, Giacomo Squicciarini, and David Thompson. Modeling the inter-
action between piano strings and the soundboard. In Proceedings of Meetings on Acoustics,
volume 49. AIP Publishing, 2022.
[35] Maarten Van Walstijn, Abhiram Bhanuprakash, and Paul Stapleton. Finite-difference sim-
ulation of linear plate vibration with dynamic distributed contact. In Sound and Music
Computing Conference 2021, pages 92–99, 2021.
[36] Maarten Van Walstijn, Vasileios Chatziioannou, and Abhiram Bhanuprakash. Implicit and
explicit schemes for energy-stable simulation of string vibrations with collisions: Refinement,
analysis, and comparison. Journal of Sound and Vibration, 569:117968, 2024.

[37] Peter Wriggers. Finite element algorithms for contact problems. Archives of computational
methods in engineering, 2:1–49, 1995.
[38] Jia Zhao, Qi Wang, and Xiaofeng Yang. Numerical approximations for a phase field dendritic
crystal growth model based on the invariant energy quadratization approach. International
Journal for Numerical Methods in Engineering, 110(3):279–300, 2017.

A      Prestrain and prestress
Consider 3 configurations of a material: natural → initial → current [28]. In the natural configu-
ration, no prestrain and prestress is present. In the initial configuration, prestrain and prestress
exist. In the current configuration, strain and stress emerge in addition to prestrain and pre-
stress. Under the same xyz coordinate system, define R3 vectors x̄, x, x̂ as the coordinates of
the same material particle in the natural, initial and current configurations respectively. The
following relation holds:

x = x̄ + ū(x̄),
x̂ = x + u(x),                                        (A.1)

where ū and u are the displacement vectors from natural to initial and from initial to current
configurations respectively. Denote deformation gradients Ẑ = ∂∂ x̄
x̂
, Z = ∂∂x
x̂
and Z̄ = ∂x
∂ x̄ ; denote
Jacobian matrices J = ∇u, J̄ = ∇ū,. We then have

Z = I + J , Z̄ = I + J̄ ,

Ẑ = Z Z̄ = (I + J ) I + J̄ .                                  (A.2)

The Green-Lagrange strain tensor of current configuration with respect to natural configuration
is
1 ⊤                    ⊤           ⊤                
Ê =     Ẑ Ẑ − I = I + J̄ (I + J ) (I + J ) I + J̄
1         ⊤    ⊤
 1
⊤
                 
J + J ⊤ + J ⊤ J I + J̄

=     J̄ + J̄ + J̄ J̄ +       I + J̄
# 2          2            
⊤            ⊤
≈ E = ϵ + Ē + J̄ ϵ + ϵJ̄ + J̄ ϵJ̄ ,
e                                                                        (A.3)
                                                         
⊤    ⊤
where Ē = 12 J̄ + J̄ + J̄ J̄ is the prestrain tensor, and ϵ = 21 J + J ⊤ is the engineering
strain discarding the second-order 12 J ⊤ J term for the sake of linearization. Note this linearization
should . Assuming J̄ = {J¯ij } is symmetric, viz. no rigid body rotation, then we have eigen
decomposition J̄ = U diag(λ1 , λ2 , λ3 )U ⊤ and
# 1 2
J̄ + J̄ = Ē
2
λ1 + 12 λ21                                                λ′1
                                                                               

U                       λ2 + 12 λ22                  U⊤ = U          λ′2          U ⊤,   (A.4)
λ3 + 21 λ23                            λ′3
p
from which λi = 4 λ′i + 4 − 8 is the solution and J̄ can be computed per eigen decomposition.
The uniqueness of this solution stems from that the principal stretches λ′i ≥ −1 should hold
for normal cases, viz. no negative compression, and λi should be close to λ′i to be consistent
with the case of ignoring 12 J̄ . If the prestrain is not large enough to induce non-negligible
⊤
geometric nonlinearity, 12 J̄ J̄ can be disregarded and J̄ = Ē simply holds. But generally for
string instruments like piano, prestrain may be large and even dominate the post-strain, thus we
should cover the geometric nonlinearity of prestrain. This, however, would not necessarily make
(A.3) nonlinear with respect to J , because strain deformation is normally small enough to make
# 1 ⊤
# 2 J J negligible. Consequently, with J̄ and Ē known, the prestrain model (A.3) is reasonably

linear with respect to the unknowns. Converting it into vector form, we find
                               
# 2 0 0 0 0 0
 0 2 0 0 0 0 
→
−    →
−       →
−     →
−           →
−                                            
 0 0 2 0 0 0 

Ê ≈ E = Ē + E dyn , E dyn = (I + Ψ2 Ψ1 ) ϵ , Ψ1 = 
e             e          e                              →
−                                            
 0 0 0 1 0 0 

 0 0 0 0 1 0 
# 0 0 0 0 0 1
¯ ¯                   ¯             ¯                ¯     ¯
J11 (J11 +2)         2             2
J12 (J11 +1)               J¯13 (J¯11 +1)                 J¯12 J¯13

J12           J13
               J¯22 (J¯22 +2)      J¯23
J¯12 (J¯22 +1)                 J¯12 J¯23              J¯23 (J¯22 +1)       
                              ¯
J    J¯
# 33 ( 33  +2 )            ¯
J   J¯                 J¯     ¯
# 13 ( 33
J    +1 )           J¯23 (J¯33 +1)

Ψ2 =                                                           13 23                                                                 
J¯11 J¯22 +J¯11 +J¯12 +J¯22 J¯11 J¯23 +J¯12 J¯13 +J¯23 J¯12 J¯23 +J¯13 J¯22 +J¯13 
                                                                                                                               

J¯11 J¯33 +J¯11 +J¯13 +J¯33 J¯12 J¯33 +J¯12 +J¯13 J¯23

Sym                                                                                          J¯22 J¯33 +J¯22 +J¯23
+J¯33
(A.5)
→
−
where E  e dyn is the dynamic part of strain vector; all the vectors here are defined similar to
(2.3). It is now straight forward that the prestrain functions a linear transformation (addition
and scaling) of the strain at a first approximation. However, as in (A.5) Ēij (x̄) is expressed in
natural coordinates, converting it into initial coordinate representations to align with → −
ϵ would
→
−             →
−        →
−
be preferrable. In order for this, define prestress vector S̄ (x̄), then Ē = D −1 S̄ . From (A.1),
→
−          →
−
an inverse mapping x̄ = f (x) should exist. If we let T (x) = S̄ (f (x)), the vectorized tension
→
−         →
−
field from (2.8), and subsititute Ē = D −1 T (x) into (A.4)(A.5), then the total strain Ê can be
expressed in only the initial configurations. The advantage of this is not only avoiding finding
the intricate natural coordinates, but also making it straightfoward to satisfy static equilibrium
by condition ∇ · T (x) = 0.
→
−
Given the strain vector Ê , the dynamic strain energy is
Z Z →
−
E
e dyn       →
−         →
−
U=                       e dyn · d E
Ψ0 S         e dyn dV
Ω   0
Z Z →
−
E
e dyn         →
−         →
−
=                      e dyn · d E
Ψ0 D E         e dyn dV
Ω 0
Z     →
−         →
−
=      Ψ0 Ee dyn · D E
e dyn dV
Z Ω
# 1 →
Ψ0 −                          →
−
                    
=         ϵ · D + Ψ⊤        ⊤
# 1 Ψ2 DΨ2 Ψ1   ϵ dV,                                                  (A.6)
Ω 2

where Ψ0 (x) = det(Z̄)−1 is the volume factor linear w.r.t. the unknowns, accounts for volume
→
−
change from natural to current configuration, and can be computed given J̄ . Note that S    e dyn
here is understood as the (dynamic) second Piola-Kirchhoff (PK2) stress that is energy conjugate
→
−
to Ee dyn . Though PK2 should be formulated in the natural configuration, we have transformed
it to the initial configuration along with the volume factor. According to [2], PK2 may also
be changed to Cauchy stress in the initial configuration, but it is unnecessary here because the
strain energy is the same even without doing so. And anyway, what we seek for here is not PK2,
but the dynamic stress vector → −
σ that is energy conjugate to →
−ϵ , which can be derived from the
expression of dynamic strain energy as
→
−
σ̂ = D →−ϵ +→
−τ , →
 −                        →
−
τ + Ψ0 Ψ⊤    ⊤
# 1 Ψ2 DΨ2 Ψ1 ϵ ,                     (A.7)

where →−τ is the dynamic prestress vector. It is now clear that the effect of prestress can be seen
as adding a symmetrically transformed constitutive matrix Ψ⊤             ⊤
# 1 Ψ2 DΨ2 Ψ1 .
Finally, we acknowledge that tn the presence of large deformations from initial to current
configuration, the quadratic term in the Green-Lagrange strain should be retained, leading to
nonlinear operators on the unknowns for the strain energy, as well as PDEs and ODEs. However,
this concern is irrelevant in case of large deformations from natural to initial configuration because
even if prestrain itself is nonlinear, it is linear w.r.t. the strain where the unknowns are contained

in. Another more intricate nonlinearity, often known as the geometric nonlinearity, arises if we
want to express stress and prestress in the current configuration. This would require transforming
PK2 to Cauchy stress, yet we have not encountered the necessity for doing so in the present
context.

B     Lagrangian formulation
As a complement to the Newtonian formulation, we present a non-conservative Lagrangian for-
mulation of the 3D elastic material model. To derive system ODEs (weak form) in a more
straightforward way, FEM space discretization is incorporated into the Lagrangian formulation,
skipping the intermediate PDEs (strong form). As per (2.17), the kinetic energy and its variation
over [t0 , t1 ] are
Z                      Z
# 1 1
Ek =        ρu̇ · u̇dV =         ξ̇ · ρP ⊤ P ξ̇dV
Ω 2                 2 Ω
Z t1          Z t1 Z                              Z t1 Z
δEk dt =         δ ξ̇ · ρP ⊤ P ξ̇dV dt = −          δξ · ρP ⊤ P ξ̈dV dt (B.1)
t0             t0       Ω                               t0   Ω

where integration by parts is utilized assuming

δξ(t0 ) = δξ(t1 ) = 0.                         (B.2)

Note that we have excluded ξ 0 in the derivation of kinetic energy because nonzero ξ 0 (boundary
displacements) actually implys the existence some non-conservative forces (e.g. constraints).
Nevertheless, we can compute the “virtual kinetic energy” as
Z t1          Z t1 Z
δEk,0 =         δ ξ̇ · ρP ⊤ P 0 ξ̇ 0 dV dt                (B.3)
t0                   t0   Ω

The potential energy and its variation are
Z
Ep = −        u · Div (σ + τ ) dV
Ω 2
Z         3
!       Z      3
!
1X                                1X
=−                ui (σ i + τ i ) · dΓ +           ∇ui · (σ i + τ i ) dV
Γ    2 i=1                           Ω 2 i=1
Z "X  3
#
1        ⊤

=                ξ · Qi (Ai + B i ) Qξ dV
Ω i=1 2
Z
=       ξ · Q⊤ (A + B) QξdV
Ω 2
Z
δEp =      δξ · Q⊤ (A + B) QξdV                                                    (B.4)
Ω

where Dirichlet or Neumann boundary conditions should automatically apply to ensure the
integral on the boundary is zero. Similar to the kinetic energy, nonzero ξ 0 does not correspond
to a potential energy but a “virtual potential energy” (equivalent to virtual work but sign flipped
here for convenience) as               Z
δEp,0 =             δξ · Q⊤ (A + B) Q0 ξ 0 dV                 (B.5)
Ω
As for the damping force which is non-conservative, we can compute its virtual work as
Z
δWd = 2µ      δu · Div (σ̇ + τ̇ ) dV
Ω
Z "X  3
#
= −2µ          δ∇ui · (σ̇ i + τ̇ i ) dV
Ω    i=1
Z            h                                  i
= −2µ            δξ · Q⊤ (A + B) Qξ̇ + Q⊤ (A + B) Q0 ξ˙0 dV.           (B.6)
Ω

It can be verified that the virtual work approach is also applicable to conservative forces like
stress and prestress to derive the identical potential energy functions. For other non-conservative
forces, the virtual work is
Z                Z
δWnc =      δu · F dV =     δξ · P ⊤ F dV.                      (B.7)
Ω                   Ω

Applying the principle of virtual work, we derive the energy equation
Z t1                                                    Z t1                         
(δEk + δEk,0 − δEp − δEp,0 + δWd + δWnc ) dt = −        δξ · M ξ̈ + C ξ̇ + Kξ − f dt = 0,
t0                                                                    t0
(B.8)
where M , C, K, f are defined in (2.20). Since δξ is arbitrary as long as (B.2) is satisfied, (2.19)
can be derived from (B.8).

C        Elastic material contacting rigid surface in 3D setting
We now consider the case of a material A contacting another material B in a fully 3D setting,
where A is elastic with weak form ODEs like (2.19) and B’s contact surface is rigid (hence B may
still be an elastic material). In this case, applying Dirichlet boundary conditions through ξ 0 (t)
would be infeasible because not only the contact status (in contact or not) but also the contact
boundary changes with time. As we shall see, the potential contact boundary is Dirichlet when
contact is present meaning that boundary stress and strain appear, and is Neumann when contact
is absent meaning that boundary stress and strain vanish. This leads to a time-varying dimension
(number of DOFs) of the system ODEs, invalidating the modal superposition method. To model
the dynamic evolution of contact system, we adopt the master-slave idea and will brief describe
our understanding and application of it. Readers can refer to [37][24] for a comprehensive and
rigorous FEM formulation of the contact problem.
Consider a system of second-order ODEs where the rhs source term depends on the DOFs:
              
M ξ̈(t) + C ξ̇(t) + Kξ(t) = f (t), f (t) = r t, ξ(t), ξ̇(t) .          (C.1)

To derive time stepping algorithms like those in 7.2, we first integrate (C.1) over [tn , tn+1 ] to
yield
h                 i                             Z tn+1          Z tn+1
M ξ̇(tn+1 ) − ξ̇(tn ) = −C [ξ(tn+1 ) − ξ(tn )] − K        ξ(t)dt +        f (t)dt
tn              tn
h               i                            h                                  h2
M ξ̇(tn+1 ) − ξ̇(tn ) ≈ −C [ξ(tn+1 ) − ξ(tn )] − K [ξ(tn ) + ξ(tn+1 )] + hf (tn ) + ḟ (tn ).
# 2 2
(C.2)

Combining this with
Z tn+1
hh                   i
ξ(tn+1 ) − ξ(tn ) =            ξ̇(t)dt ≈     ξ̇(tn ) + ξ̇(tn+1 ) ,     (C.3)
tn                  2

the update scheme for DOFs vector is found to be

h2
                                                   
ξ(tn+1 ) = Z −1
# 1 Z 0 ξ(t n ) + 2M ξ̇(t n ) + hf (tn ) +    ḟ (t n )  ,
# 2 2
ξ̇(tn+1 ) = ξ(tn+1 ) − ξ(tn ) − ξ̇(tn ),                                     (C.4)
h             h
where
# 2 h
Z0 =   M + C − K,
h        2
# 2 h
Z 1 = M + C + K.                                     (C.5)
h        2
It now suffices to describe the time stepping algorithm for contact problem as follows:

1. At time step tn , set iteration steps j = 1 and jmax > 1; set ξ aj (tn ) = ξ j (tn ) = ξ(tn ),
ξ˙aj (tn ) = ξ̇ j (tn ) = ξ̇(tn ), f aj (tn ) = f˙aj (tn ) = 0, M aj = M , C aj = C, K aj = K. Here we
assume the rhs source only comes from contact between A and B. If any other rhs sources
are present and independent of this contact, add them to f aj (tn ) and f˙aj (tn ).

2. Input ξ aj (tn ), ξ˙aj (tn ), f aj (tn ), f˙aj (tn ), M aj , C aj , K aj into (C.4) to compute ξ aj (tn+1 ) and
ξ˙aj (tn+1 ).

3. Based on ξ aj (tn+1 ) and ξ˙aj (tn+1 ), check if A “penetrates” B at any potential contact bound-
ary nodes using a geometry searching algorithm. Here ξ aj (tn+1 ) and ξ˙aj (tn+1 ) are used to
compute A’s surface forces acting on B, so as to obtain B’s displacements at tn+1 .
(a) If penetration occurs at some nodes:
i. Set j ← j + 1.
A. If j > jmax , go to step 4.
B. Otherwise, reclassify ξ j (tn ), ξ̇ j (tn ) (all DOFs, dimension N ) into ξ aj (tn ),
˙
ξ˙aj (tn ) (non-penetrated DOFs, dimension Nja and ξ bj (tn ), ξ bj (tn ) (penetrated
˙
DOFs, dimension Njb ). Note here ξ bj−1 (tn ), ξ bj−1 (tn ) (if any) must be classified
˙
as penetrated in ξ bj (tn ), ξ bj (tn ). Accordingly, M , C, K are adjusted to M aj ,
C j , K j with dimension Nja , in a way as if ξ aj were ξ (unknown DOFs) and ξ bj
a      a

˙
were ξ 0 (known DOFs) in (2.20). Using ξ bj (tn ) and ξ bj (tn ) (Dirichlet boundary
conditions), f aj (tn ) and f˙aj (tn ) with dimension Nja can be computed.
ii. Input ξ aj (tn ), ξ˙aj (tn ), f aj (tn ), f˙aj (tn ), M aj , C aj , K aj into (C.4) to compute ξ aj (tn+1 )
and ξ˙a (tn+1 ).
j

iii. Based on ξ aj (tn+1 ) and ξ˙aj (tn+1 ), check if A “penetrates” B at any potential
contact boundary nodes using a geometry searching algorithm. Here ξ aj (tn+1 )
and ξ˙aj (tn+1 ) are used to compute A’s surface forces acting on B, so as to obtain
˙                            ˙
B’s displacements at tn+1 , as well as ξ bj (tn+1 ) and ξ bj (tn+1 ). Note here if ξ bj (tn+1 )
is not well-defined due to non-smoothness, the weak derivative may be adopted as
˙
an alternative. Combine ξ aj (tn+1 ), ξ˙aj (tn+1 ) and ξ bj (tn+1 ), ξ bj (tn+1 ) to get ξ j (tn+1 )
and ξ̇ j (tn+1 ).
A. If penetration occurs at some nodes, go back to step 3.(a).i.
B. Otherwise, go to step 4.
(b) Otherwise, set ξ (tn+1 ) = ξ a (tn+1 ), ξ̇ (tn+1 ) = ξ˙a (tn+1 ).
j               j            j              j

4. End the current time step with ξ(tn+1 ) = ξ j (tn+1 ) and ξ̇(tn+1 ) = ξ̇ j (tn+1 ), and move on
to the next time step.
The basic idea of this algorithm is to try Neumann first, and if penetration occurs then change to
Dirichlet. This strategy can be justified from an energy perspective: first seek for displacements
minimizing the Lagrangian and virtual work of the system without any constraint; if unrealistic
penetration occurs, add an equality constraint regarding Dirichlet boundary displacements, so
as to find displacements minimizing the Lagrangian and virtual work under contact constraint
(similar to the Lagrange multipliers). This check of penetration can lead to the nonlinearity
and non-smoothness of a contact problem. For geometry searching algorithms needed to check
penetration, we refer to the [COMSOL description]. If the contact boundary is assumed to be
only a fixed point, then only one or two iterations per time step is needed.
Certain contact problems may involve deformations large enough to induce non-negligible
nonlinearity. For nonlinearity of strain, the Green-Lagrange strain instead of the engineering
strain may be needed. For nonlinearity of stress, the second-Piola–Kirchhoff stress tensor instead
of the Cauchy stress tensor may be needed. This would introduce another kind of nonlinearity
into the contact model. While the nonlinearity due to contact’s one-sided nature still keeps the
computation of each iteration in one time step linear, the nonlinearity due to nonlinear stress
and strain can make the PDEs as well as ODEs nonlinear, meaning that computation of each

iteration is nonlinear if no linearization is applied. If the nonlinear perturbation to the equations
is small (controlled by some “small parameters”), the perturbation method applying linearization
several times is acceptable; otherwise, the perturbation method would suffer significant accuracy
loss.

D     An energy-stable scheme for nonlinear forces
To better solve elastic solid problems subject to deformation nonlinearity, particularly elastic
contact and piano string vibration, we are working on integrating into our model the invariant
energy quadratization (IEQ) method [38] and scalar auxiliary variable (SAV) method [29, 30].
Based on the idea of IEQ, SAV is a recent advancement in computational physics towards efficient
and energy-stable time discretization scheme for gradient flows. Both methods have been applied
to musical instruments modeling, achieving discrete energy evolution (conservation or dissipation)
that is consistent with the continuous counterpart, using fully explicit or linearly implicit update
schemes [18, 36, 6]. These schemes attain the desired discrete energy-stable property while
being more efficient than existing nonlinear solvers many of which rely on Newton-Raphson like
iterations with computationally expensive Jacobian matrices.
To introduce nonlinearity into the 3D elastic solid model, we rewrite (2.19) as

M ξ̈(t) + C ξ̇(t) + Kξ(t) + g [ξ(t)] = f (t),                      (D.1)

where g(ξ) is an operator incorporating all the “symmetric” nonlinear forces of the system, which
can be conservative or non-conservative. The symmetry property we require here means that g
is a self-adjoint operator thus the variational Jacobian matrix

J [ξ(t)] = δξ g                                (D.2)

is symmetric. This is equivalent to g being a self-adjoint operator. For simplicity we have not
considered nonlinear damping forces in the form of g 1 (ξ̇) here, but they can simply be treated in
a similar way to g(ξ). An example of nonlinear symmetric forces is the nonlinear stress arising
from the quadratic term in Green-Lagrange strain, as well as the damping force proportional to
its first-order time derivative. Anyway, linear or nonlinear forces not satisfying this symmetry
property are incorporated in f (t), and unfortunately the coupling forces in our piano model are
probably such examples. The symmetry property has important implications that one can define
a nonlinear energy functional as
Z t
Enl [ξ(t)] =     ξ̇(t′ ) · g [ξ(t′ )] dt′ ,                 (D.3)

with first-order time derivative

∂t Enl [ξ(t)] = ξ̇(t) · g [ξ(t)] ,                     (D.4)

and variation
Z t                  Z t
′
δEnl [ξ(t)] =                    ξ̇ · δgdt′ ,
δ ξ̇ · gdt +
# 0 0
Z t             Z t
′
= δξ(t) · g − δξ(0) · g +       ξ̇ · δgdt −     δξ · ġdt′
# 0 0
= δξ(t) · g                                                       (D.5)

where δξ(0) = 0 is imposed and one can derive ξ̇ · δg − δξ · ġ = 0 with the symmetry property of
J . It is now clear that under symmetry, the virtual work equates real work. The kinetic energy
functional is
Ek [ξ(t)] =     ξ̇(t) · M ξ̇(t),
∂t Ek [ξ(t)] = ξ̇(t) · M ξ̈(t).                        (D.6)

The linear potential energy functional is
Ep [ξ(t)] =     ξ(t) · Kξ(t),
∂t Ep [ξ(t)] = ξ̇(t) · Kξ(t).                              (D.7)

The overall energy evolution of the system is found to be
                                    
ξ̇(t) · M ξ̈(t) + C ξ̇(t) + Kξ(t) + g [ξ(t)] = ξ̇(t) · f (t)
∂t E = ∂t Ek + ∂t Ep + ∂t Enl = ξ̇(t) · f (t) − ξ̇(t) · C ξ̇(t)            (D.8)

where E[ξ(t)] = Ek + Ep + Enl .
To apply SAV [29], assume the nonlinear potential energy functional can be rewritten in the
square of a scalar variable ψ(t) as
Enl [ξ(t)] =      ψ(t)2 ,                              (D.9)
where
√ p
ψ(t) =    2 Enl [ξ(t)] + C0 ,                                  (D.10)
h                  i
# 1 ξ̇(t) · g   a(t), ξ(t), ξ̇(t)
ψ̇(t) = √       p                          ,                   (D.11)
# 2 Enl [ξ(t)] + C0

and Enl [ξ(t)] ≥ −C0 is assumed (bounded from below). Utilizing the Crank–Nicolson scheme
described in [30] we have
Z tn+1           Z tn+1
# 1 ψ(t)
gdt = √           p             g [ξ(t)] dt
tn           2  tn       E nl [ξ(t)]
h     ψ(tn ) + ψ(tn+1 )        h           i
≈ √ r                            g   ξ(t n+ 1)
# 2 2         h         i                   2
Enl ξ(tn+ 12 ) + C0
                
h           ψ(tn ) + ψ(tn+1 )                         h
≈ √ q                                           g ξ(tn ) + ξ̇(tn )
# 2 2 E [ξ(t )] + h Ė [ξ(t )] + C                        2
nl     n      2 nl        n        0

h                                       
= (ψ(tn ) + ψ(tn+1 )) s ξ(tn ), ξ̇(tn ) ,                           (D.12)
where                                                     h                   i
                                      g ξ(tn ) + h2 ξ̇(tn )
s ξ(tn ), ξ̇(tn ) : RN = √ q                                            .      (D.13)
# 2 Enl [ξ(tn )] + h2 ξ̇(tn ) · g [ξ(tn )] + C0
Here we encountered the problem of how to approximate the values of g and Enl at mid step
tn+ 12 . Unlike the several interpolation methods used in [30], we chose a more direct first-order
approximation approach using Ėnl [ξ(tn )] and ξ̇(tn ). ġ(ξ(tn )) was not used because its expression
involving J may be hard to write, and we instead did a slightly wrose approximation using only
ξ̇(tn ); it is also viable to approximate ġ(ξ(tn )) using g(ξ(tn−1 )) and g(ξ(tn )). Notice that in
(D.12) ψ(tn+1 ) is unknown, so we need to discretize (D.11) for another update scheme
           
# 1 (ξ(tn+1 ) − ξ(tn )) · g ξ(tn+ 12 )
ψ(tn+1 ) − ψ(tn ) ≈ √           r
# 2 h         i
Enl ξ(tn+ 12 )
               
≈ (ξ(tn+1 ) − ξ(tn )) · s ξ(tn ), ξ̇(tn ) .          (D.14)

It now sufficies to formulate the whole discretization of nonlinear ODEs as
h                 i                           Z tn+1       Z tn+1            Z tn+1
M ξ̇(tn+1 ) − ξ̇(tn ) + C [ξ(tn+1 ) − ξ(tn )] + K        ξdt +        g [ξ] dt =        f dt (D.15)
tn             tn             tn

Combining this with SAV and
hh                   i
ξ(tn+1 ) − ξ(tn ) ≈    ξ̇(tn ) + ξ̇(tn+1 )
# 2 2
ξ̇(tn+1 ) ≈ ξ(tn+1 ) − ξ(tn ) − ξ̇(tn ),                                (D.16)
h             h
we obtain the major update scheme:
                               
# 2 2                                             h
M       ξ(tn+1 ) − ξ(tn ) − 2ξ̇(tn ) + C [ξ(tn+1 ) − ξ(tn )] + K [ξ(tn ) + ξ(tn+1 )]
h            h                                             2
h                                     h
+ hψ(tn )s + ss⊤ [ξ(tn+1 ) − ξ(tn )] = hf (tn ) + ḟ (tn )                          (D.17)
2                                     2
h2

ξ(tn+1 ) = Z −1
# 1 Z 0 ξ(tn ) + 2M ξ̇(tn ) + hf (tn ) + ḟ (tn ) ,                    (D.18)

where
# 2 h   h
Z0 =   M + C − K + ss⊤ ,
h        2   2
# 2 h   h ⊤
Z 1 = M + C + K + ss .                                                   (D.19)
h        2   2
Therefore, the SAV update scheme consists of three steps:
1. Input ξ(tn ), ξ̇(tn ), ψ(tn ) into (D.18) to compute ξ(tn+1 ).
2. Input ξ(tn ), ξ̇(tn ), ξ(tn+1 ) into (D.16) to compute ξ̇(tn+1 ).

3. Input ξ(tn ), ξ̇(tn ), ξ(tn+1 ), ψ(tn ) into (D.14) to compute ψ(tn+1 ).
It can be seen that the major update scheme (D.18) is linear w.r.t. ξ(tn+1 ), allowing for efficient
time stepping. However, we notice that ss⊤ in (D.19) is a fully dense N × N matrix that may
be infeasible to store and solve for large number DOFs. This problem can be well alleviated by
performing eigen decomposition only once to the time-invariant matrix
# 2 h
Z2 =      M + C + K, Z −1     H
# 2 ≈ ΦΛΦ ,                                              (D.20)
h        2
where the dimension of Φ and Λ is M ≪ N , as only the smallest M eigenvalues and corresponding
eigenvectors are needed. This should be regarded as a generalized eigenvalue problem because
it is desired to avoid explicit matrix inversion. Then by the Shermann–Morrison–Woodbury
formula we have
             −1                       −1
−1           h ⊤            −1     2    ⊤ −1
 −1 ⊤
Z 1 = Z 2 + ss           = Z2 −        + s Z2 s       Z −1
# 2 s   Z2 s ,           (D.21)
# 2 h

where Z −1                                                                                    −1
# 2 can be approximated by the truncated eigenvalues and eigenvectors. In this way Z 1
multiplying a vector can be efficiently computed without storing and inverting a N × N dense
matrix.
To investigate the energy property of this SAV scheme, we multiply h1 [ξ(tn+1 ) − ξ(tn )] =
# 1 10
# 2 [ξ̇(tn ) + ξ̇(tn+1 )] on both sides of (D.17) to yield

# 1 1                     1
ξ̇(tn+1 ) · M ξ̇(tn+1 ) − ξ̇(tn ) · M ξ̇(tn ) + [ξ(tn+1 ) − ξ(tn )] · C [ξ(tn+1 ) − ξ(tn )]
# 2 2                     h
# 1 1                     1            1
+ ξ(tn+1 ) · Kξ(tn+1 ) − ξ(tn ) · Kξ(tn ) + ψ(tn+1 )2 − ψ(tn )2
# 2    2                    2            2
h
= [ξ(tn+1 ) − ξ(tn )] · f (tn ) + ḟ (tn ) .                                                  (D.22)
# 10 The reason for the equality 1 [ξ(t                  1
h     n+1 ) − ξ(tn )] = 2 [ξ̇(tn ) + ξ̇(tn+1 )] to hold here is that this equation is the
actual numerical update scheme we use, no matter how approximate it is to the true continuous values.

Combining this with the definition of E, Ek , Ep , Enl , we obtain the evolution of discrete energy
as
                  
h          1                   2
E [ξ(tn+1 )] − E [ξ(tn )] = [ξ(tn+1 ) − ξ(tn )] · f (tn ) + ḟ (tn ) − ∥ξ(tn+1 ) − ξ(tn )∥C , (D.23)
# 2 h
where ∥x∥A = x⊤ Ax. Since C is normally positive definite, the discrete energy is guaranteed
to dissipate if g is conservative and energy injection through f (t) becomes zero. Therefore, it
can be concluded that the SAV scheme applied here is energy stable at least for kinetic energy,
potential energy, nonlinear “symmetric” energy, and damping.
However, it seems energy stablility may fail to satisfy for the non-symmetric forces f (t), which
we should actually write as an operator f [ξ(t), ξ̇(t), ξ̈(t)]. These non-symmetric “endogenous”
forces often relate to the coupling between systems. For example, for the soundboard system,
f is the string’s surface force exerted on the soundboard. If f is fully linear with respect to
the unknowns, then we can use tn+1 values in f to construct linearly implicit schemes which
effectively avoids energy unstability, though the formulas may be cumbersome to write. But if
f exhibits nonlinearity and tn+1 values are used to avoid energy unstability, the scheme would
become nonlinearly implicit and it seems only implicit methods, e.g. backward differentiation
formula (BDF), can solve for the unknown DOFs. We thus conclude that SAV does not address
well the non-symmetric forces, which is vital for modeling the nonlinear coupling between the
distinct subsystems of the piano. A tradeoff is to keep using energy-unstabe linear schemes for
the nonlinear non-symmetric forces, but perform several iterations as proposed in section 7.2 to
compensate. We hope to see more developments regarding efficient and energy-stable schemes
for nonlinear inter-system coupling.
We now consider an example of nonlinear potential energy arising from large deformations.
For a 3D elastic solid with large deformations, a major source of nonlinearity is the quadratic
term in the Green-Lagrange strain. The tensor and vector versions of this nonlinear strain (linear
parts omitted) are
1X                 ⊤
ϵnl (x, t) =         (∇ui ) (∇ui ) ,                              (D.24)
# 2 i=1
        1          2      
# 2 (∂x ui )
# 1 2
# 2 (∂y ui )
               
# 3                 
⇒→
−                                              2 (∂z ui )
X                  
ϵ nl (x, t) = A (u(x, t)) =                    
 (∂ u ) (∂ u )                      (D.25)
i=1    x i    y i 
 (∂ u ) (∂ u ) 
x i      z i
(∂y ui ) (∂z ui )

where A : R3 → R6 is a nonlinear operator. From the nonlinear strain, we find the conservative
part of nonlinear strain energy functional as
Z
Enl [u(x, t)] =     A (u(x, t)) · DA (u(x, t)) dV.             (D.26)
# 2 Ω
Taking gradient of this functional, the nonlinear stress (body force version) is derived as
δEnl [u]
∇ · σ nl =            .                            (D.27)
δu
Besides Enl , there may exist damping proportional to the nonlinear stress, leading to the non-
conservative part of nonlinear strain energy functional as
Z tZ
′
Enlnc [u(x, t ), t] =      Ȧ (u(x, t′ )) · 2µD Ȧ (u(x, t′ )) dV dt′ , (D.28)
# 0 Ω

where 2µ is the viscous damping coefficient. The dynamics of energy dissipation can then be
revealed as                                 Z
′
∂t Enlnc [u(x, t ), t] =   Ȧ (u(x, t)) · 2µD Ȧ (u(x, t)) dV.    (D.29)
Ω
However, consideration of nonlinear damping seems lacking in existing IEQ and SAV research.
This may stem from that energy dissipation is path-dependent thus the integration over [0, t] can

not be eliminated. This would require some additional but still acceptable computation costs,
and has been covered by our nonlinear energy model D.3. In existing research applying IEQ or
SAV to musical instruments simulation, there can be no damping [36] or linear damping [16].
Now we apply space discretization to the nonlinear strain energy functional. Based on (2.15)
(2.17) we should be able to find
→
−
ϵ nl (x, t) = A (x, ξ(t)) + A0 (x, ξ(t), ξ 0 (t)) ,                (D.30)

where A and A0 are (R3 , RN ) → R6 and (R3 , RN , RN0 ) → R6 nonlinear functions (a narrower
concept than operator) respectively; A accounts for the conservative part and A0 accounts for
the non-conservative part when ξ 0 is nonzero. For simplicity, the detailed expression of A and
A0 are currently not presented here. The nonlinear strain energy functional is then
Z
Enl [ξ(t)] =     A (x, ξ(t)) · DA (x, ξ(t)) dV,                   (D.31)
# 2 Ω

and its variation and first derivative are

δEnl [ξ(t)] = δξ · B (ξ(t)) ,
∂t Enl [ξ(t)] = ξ̇(t) · B (ξ(t)) ,
Z
⊤
B (ξ) =       [∇ξ A (x, ξ)] DA (x, ξ) dV                     (D.32)
Ω

Since A0 relates to ξ 0 (t), i.e. nonzero Dirichlet boundary conditions, it does not satisfy the sym-
metry (self-adjoint) condition previously highlighted. We thereby seek to compute the nonlinear
virtual strain energy as

δEnl,0 [ξ(t)] = δξ · B 0 (ξ(t), ξ 0 (t))
Z
⊤
B 0 (ξ, ξ 0 ) =     [∇ξ A (x, ξ)] DA0 (x, ξ, ξ 0 ) dV
Ω
Z
⊤
+     [∇ξ A0 (x, ξ, ξ 0 )] D [A (x, ξ) + A0 (x, ξ, ξ 0 )] dV    (D.33)
Ω

Finally, we add B to g and add −B 0 to f in the nonlinear ODEs (D.1). Note that since
the nonlinear stress belongs to conservative forces, its corresponding nonlinear strain energy
functional (D.31) has eliminated the integration over [0, t]. This makes it simple to compute the
Enl [ξ(tn )] in (D.13).
