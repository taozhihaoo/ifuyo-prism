import io

p = 'crates/prism-impact/src/traversal.rs'
s = io.open(p, encoding='utf-8').read()

# 1) index unresolved/ambiguous module deps by raw_target (counting only)
old = '''    /// 名字/限定名 → 指向该名字的未解析调用边（§20 deleted-symbol 专用）。
    pub unresolved_callers_by_name: BTreeMap<String, Vec<ReverseEdge>>,'''
new = '''    /// 名字/限定名 → 指向该名字的未解析调用边（§20 deleted-symbol 专用）。
    pub unresolved_callers_by_name: BTreeMap<String, Vec<ReverseEdge>>,
    /// raw_target → 未解析/歧义模块依赖边（仅计数/诊断用，不遍历，§5）。
    pub unresolved_dependents_by_target: BTreeMap<String, Vec<ReverseEdge>>,'''
assert old in s
s = s.replace(old, new)

old = '''        for dependency in &graph.dependencies {
            if dependency.resolution != ResolutionStatus::Resolved {
                continue;
            }'''
new = '''        for dependency in &graph.dependencies {
            if dependency.resolution != ResolutionStatus::Resolved {
                // 未解析/歧义模块依赖：仅进计数索引（§44/§102/§103），
                // 绝不作为遍历边（否则伪造影响目标）。
                let relation = if dependency.kind == DependencyKind::ProjectReference {
                    RelationKind::ProjectReferences
                } else {
                    RelationKind::DependsOn
                };
                index
                    .unresolved_dependents_by_target
                    .entry(dependency.raw_target.clone())
                    .or_default()
                    .push(ReverseEdge {
                        from: dependency.source_id.as_str().to_string(),
                        relation,
                        resolution: match dependency.resolution {
                            ResolutionStatus::Ambiguous => "ambiguous".to_string(),
                            _ => "unresolved".to_string(),
                        },
                        confidence: ImpactConfidence::Probable,
                        from_path: dependency.source_path.clone(),
                        from_qualified: String::new(),
                    });
                continue;
            }'''
assert old in s
s = s.replace(old, new)

# 2) seed counting: also consult unresolved_dependents_by_target for module seeds
old = '''    match seed.key.kind {
        NodeKind::FileModule | NodeKind::Package | NodeKind::Namespace | NodeKind::Project => {
            if let Some(list) = index.module_dependents.get(&seed.key.id) {
                edges.extend(list.iter());
            }
        }'''
new = '''    match seed.key.kind {
        NodeKind::FileModule | NodeKind::Package | NodeKind::Namespace | NodeKind::Project => {
            if let Some(list) = index.module_dependents.get(&seed.key.id) {
                edges.extend(list.iter());
            }
            // 未解析模块依赖按 raw_target 对齐（仅当恰好等于项目相对路径，
            // 保守：不对 specifier 做猜测解析）。
            if let Some(list) = index.unresolved_dependents_by_target.get(&seed.path) {
                edges.extend(list.iter());
            }
        }'''
assert old in s
s = s.replace(old, new)

# 3) path reconstruction: STOP at distance-0 seeds (cycle safety) + hop cap
old = '''    // ---- 证据路径重建：沿主前驱（最短）回溯到 distance 0（§30）----
    for (state_key, accumulator) in output.impacted.clone() {
        let mut hops: Vec<PathHop> = Vec::new();
        let mut walk = Some(accumulator.key.clone());
        while let Some(current) = walk {
            let Some(state) = states.get(&current.state_key()) else {
                break;
            };
            match state.predecessors.first() {
                Some((prev, hop)) => {
                    hops.push(hop.clone());
                    walk = Some(prev.clone());
                }
                None => break,
            }
        }
        hops.reverse();'''
new = '''    // ---- 证据路径重建：沿主前驱（最短）回溯到 distance 0（§30）----
    // 环安全（§80）：只从 distance ≥ 1 的节点回溯，到达 distance 0 即停；
    // 跳数上限 = max_depth + 1，任何环都不可能走成无限路径。
    for (state_key, accumulator) in output.impacted.clone() {
        let mut hops: Vec<PathHop> = Vec::new();
        let mut walk = Some(accumulator.key.clone());
        while let Some(current) = walk {
            let Some(state) = states.get(&current.state_key()) else {
                break;
            };
            if state.distance == 0 || hops.len() > options.max_depth as usize + 1 {
                break;
            }
            match state.predecessors.first() {
                Some((prev, hop)) => {
                    hops.push(hop.clone());
                    walk = Some(prev.clone());
                }
                None => break,
            }
        }
        hops.reverse();'''
assert old in s
s = s.replace(old, new)

io.open(p, 'w', encoding='utf-8', newline='\n').write(s)
print('all 3 fixes applied')
