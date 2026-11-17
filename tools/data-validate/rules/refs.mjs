// Cross-file reference integrity and id uniqueness.

/** @param {{kind:string,file:string,doc:object}[]} files  the whole data set */
export function checkRefs(files) {
  const errors = [];
  const ids = { weapon: new Set(), role: new Set(), loadout: new Set(), vehicle: new Set() };
  const byKind = (k) => files.filter((f) => f.kind === k && f.doc && typeof f.doc === 'object');

  // explosives and melee weapons are carried in loadout weapon slots: they share the weapon id space
  const idKinds = { weapon: ['weapon', 'explosive', 'melee'], role: ['role'], loadout: ['loadout'], vehicle: ['vehicle'] };
  for (const k of Object.keys(ids)) {
    for (const f of idKinds[k].flatMap((kk) => byKind(kk))) {
      const id = f.doc.id;
      if (typeof id !== 'string') continue;
      if (ids[k].has(id)) {
        errors.push({ file: f.file, pointer: '/id', message: `duplicate ${k} id: ${id}` });
      }
      ids[k].add(id);
    }
  }

  const loadoutsById = new Map(byKind('loadout').map((f) => [f.doc.id, f.doc]));

  for (const f of byKind('loadout')) {
    const d = f.doc;
    const wref = (id, pointer) => {
      if (typeof id === 'string' && !ids.weapon.has(id)) {
        errors.push({ file: f.file, pointer, message: `unknown weapon id: ${id}` });
      }
    };
    wref(d.primary, '/primary');
    wref(d.secondary, '/secondary');
    wref(d.melee, '/melee');
    wref(d.gadget_1?.id, '/gadget_1/id');
    wref(d.gadget_2?.id, '/gadget_2/id');
    if (typeof d.role === 'string' && !ids.role.has(d.role)) {
      errors.push({ file: f.file, pointer: '/role', message: `unknown role id: ${d.role}` });
    }
  }

  for (const f of byKind('role')) {
    const ov = f.doc.faction_overrides;
    if (!ov || typeof ov !== 'object') continue;
    for (const [faction, o] of Object.entries(ov)) {
      (o?.loadout_ids ?? []).forEach((lid, i) => {
        const pointer = `/faction_overrides/${faction}/loadout_ids/${i}`;
        const l = loadoutsById.get(lid);
        if (!ids.loadout.has(lid)) {
          errors.push({ file: f.file, pointer, message: `unknown loadout id: ${lid}` });
        } else if (l.role !== f.doc.id || l.faction !== faction) {
          errors.push({
            file: f.file,
            pointer,
            message: `loadout ${lid} is declared for role '${l.role}' faction '${l.faction}', not role '${f.doc.id}' faction '${faction}'`,
          });
        }
      });
    }
  }
  return errors;
}
