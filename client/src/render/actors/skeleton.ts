import { TransformNode } from '@babylonjs/core/Meshes/transformNode';
import { SOCKET, phName } from './names';

/** Canonical Humanoid joint names, in parent-first order. These are pivots, not skinning bones. */
export const JOINT_PARENT = {
  Hips: null,
  Spine: 'Hips',
  Chest: 'Spine',
  Neck: 'Chest',
  Head: 'Neck',
  RightUpperArm: 'Chest',
  RightLowerArm: 'RightUpperArm',
  RightHand: 'RightLowerArm',
  LeftUpperArm: 'Chest',
  LeftLowerArm: 'LeftUpperArm',
  LeftHand: 'LeftLowerArm',
  RightUpperLeg: 'Hips',
  RightLowerLeg: 'RightUpperLeg',
  RightFoot: 'RightLowerLeg',
  LeftUpperLeg: 'Hips',
  LeftLowerLeg: 'LeftUpperLeg',
  LeftFoot: 'LeftLowerLeg',
} as const;
export type HumanoidJoint = keyof typeof JOINT_PARENT;

const SOCKET_PARENT = {
  [SOCKET.head]: 'Head',
  [SOCKET.handR]: 'RightHand',
  [SOCKET.handL]: 'LeftHand',
  [SOCKET.back]: 'Chest',
} satisfies Record<string, HumanoidJoint>;

/** Logical, actor-local names are keys; Babylon node names remain PH_ requirement-bound. */
export class PlaceholderSkeleton {
  readonly joints: Readonly<Record<HumanoidJoint, TransformNode>>;
  readonly sockets: Readonly<Record<keyof typeof SOCKET_PARENT, TransformNode>>;

  constructor(root: TransformNode, req: string, id: number, radius: number) {
    const scene = root.getScene();
    const make = (name: string, parent: TransformNode): TransformNode => {
      const node = new TransformNode(phName(req, `${id}_${name}`), scene);
      node.parent = parent;
      return node;
    };
    const joints = {} as Record<HumanoidJoint, TransformNode>;
    for (const name of Object.keys(JOINT_PARENT) as HumanoidJoint[]) {
      const parent = JOINT_PARENT[name];
      joints[name] = make(name, parent ? joints[parent] : root);
    }
    this.joints = joints;
    const sockets = {} as Record<keyof typeof SOCKET_PARENT, TransformNode>;
    for (const name of Object.keys(SOCKET_PARENT) as (keyof typeof SOCKET_PARENT)[]) {
      sockets[name] = make(name, joints[SOCKET_PARENT[name]]);
    }
    this.sockets = sockets;
    // Keep the existing grey-box grip positions in chest space in the neutral pose.
    const r = radius;
    joints.RightUpperArm.position.set(r * 0.9, 0, 0);
    joints.RightLowerArm.position.set(0, -r * 0.2, r * 0.4);
    joints.RightHand.position.z = r * 0.8;
    joints.LeftUpperArm.position.set(-r * 0.9, 0, 0);
    joints.LeftLowerArm.position.set(r * 0.2, -r * 0.2, r * 0.8);
    joints.LeftHand.position.set(r * 0.4, 0, r * 1.6);
    joints.RightUpperLeg.position.x = r * 0.5;
    joints.LeftUpperLeg.position.x = -r * 0.5;
    this.sockets[SOCKET.back].position.z = -r;
  }

  /** Place pivots from the current capsule envelope without scaling the attachments. */
  layout(height: number, headRadius: number, bob: number): void {
    const j = this.joints;
    j.Hips.position.y = height * 0.5 + bob;
    j.Spine.position.y = height * 0.14;
    j.Chest.position.y = height * 0.14;
    j.Neck.position.y = height * 0.11;
    j.Head.position.y = height * 0.11 - headRadius * 0.7;
    j.RightLowerLeg.position.y = j.LeftLowerLeg.position.y = -height * 0.25;
    j.RightFoot.position.y = j.LeftFoot.position.y = -height * 0.25;
  }
}
