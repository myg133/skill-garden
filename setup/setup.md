# Anspire SkillGarden - Agent 瀹夎鎸囧崡

> 璁?Agent 蹇€熸帴鍏?SkillGarden 绯荤粺

---

## 蹇€熸帴鍏ワ紙5 鍒嗛挓锛?

### 绗竴姝ワ細鐢熸垚 Agent ID

```bash
# Linux/macOS
AGENT_ID="agent-$(uuidgen)"
echo "Agent ID: $AGENT_ID"

# Windows PowerShell
$AGENT_ID = "agent-" + [guid]::NewGuid().ToString()
Write-Host "Agent ID: $AGENT_ID"
```

### 绗簩姝ワ細閰嶇疆 MCP Server

鍦?Gemini CLI 閰嶇疆鏂囦欢涓坊鍔狅細

```json
{
  "mcpServers": {
    "skillgarden": {
      "command": "node",
      "args": ["path/to/mcp-server/dist/index.js"],
      "env": {
        "AGENT_ID": "$AGENT_ID",
        "MCP_SERVER_URL": "http://localhost:3000",
        "SKILLS_PATH": "path/to/skills"
      }
    }
  }
}
```

### 绗笁姝ワ細楠岃瘉杩炴帴

```bash
# 浣跨敤 MCP 宸ュ叿楠岃瘉
mcp__skillgarden__health_check
```

**棰勬湡杈撳嚭**锛?

```json
{
  "status": "ok",
  "version": "0.2.0",
  "timestamp": "2026-04-20T00:00:00Z",
  "skills_count": 3
}
```

### 绗洓姝ワ細鑾峰彇 Skills

```bash
# 鎼滅储鍙敤 Skills
mcp__skillgarden__skills_search --query "browse,review"

# 瀹夎鍩虹 Skills
mcp__skillgarden__skills_install --skill_id "browse-v1.0.0"
mcp__skillgarden__skills_install --skill_id "review-v1.0.0"
mcp__skillgarden__skills_install --skill_id "qa-v1.0.0"
```

---

## Agent 宸ヤ綔娴?

### 鏍囧噯宸ヤ綔娴?

```
鈹屸攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹?
鈹?                     Agent 浠诲姟鎵ц娴佺▼                          鈹?
鈹?                                                                鈹?
鈹? 1. 鎺ユ敹浠诲姟                                                     鈹?
鈹?    鈫?                                                           鈹?
鈹? 2. 鎷嗚В浠诲姟                                                     鈹?
鈹?    鈫?                                                           鈹?
鈹? 3. 鎼滅储 Skills                                                  鈹?
鈹?    鈹? skills_search(query)                                     鈹?
鈹?    鈫?                                                           鈹?
鈹? 4. 鏌ョ湅 Skills 缁熻                                             鈹?
鈹?    鈹? skills_stats(skill_id)  鈫?鏌ョ湅鎴愬姛鐜囥€佹墽琛屾椂闂寸瓑          鈹?
鈹?    鈫?                                                           鈹?
鈹? 5. 瀹夎 Skills                                                  鈹?
鈹?    鈹? skills_install(skill_id)                                  鈹?
鈹?    鈫?                                                           鈹?
鈹? 6. 鎵ц浠诲姟                                                     鈹?
鈹?    鈹? 浣跨敤宸插畨瑁呯殑 Skills                                       鈹?
鈹?    鈫?                                                           鈹?
鈹? 7. 璇勪环 Skills锛堢粨鏋勫寲鎸囨爣锛?                                    鈹?
鈹?    鈹? evaluate_skill(skill_id, success, duration_ms, ...)      鈹?
鈹?    鈫?                                                           鈹?
鈹? 8. 瀹屾垚浠诲姟                                                     鈹?
鈹斺攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹?
```

### 绀轰緥锛氬畬鏁翠换鍔℃祦绋?

```bash
# 1. 鎼滅储 Skills
mcp__skillgarden__skills_search --query "browse qa"

# 2. 鏌ョ湅 Skill 缁熻锛堥€夋嫨鏈€浣筹級
mcp__skillgarden__skills_stats --skill_id "browse-v1.0.0"
# 杩斿洖: { avg_success_rate: 95, avg_duration_ms: 1200, total_installs: 5, ... }

mcp__skillgarden__skills_stats --skill_id "qa-v1.0.0"
# 杩斿洖: { avg_success_rate: 88, avg_duration_ms: 3500, total_installs: 3, ... }

# 3. 瀹夎 Skills锛堥€夋嫨鎴愬姛鐜囬珮鐨勶級
mcp__skillgarden__skills_install --skill_id "browse-v1.0.0"
mcp__skillgarden__skills_install --skill_id "qa-v1.0.0"

# 4. 鎵ц娴嬭瘯锛堜娇鐢ㄥ凡瀹夎鐨?Skills锛?
# ... 鎵ц娴嬭瘯閫昏緫 ...

# 5. 璇勪环 Skills锛堟彁浜ょ粨鏋勫寲鎸囨爣锛?
mcp__skillgarden__evaluate_skill \
  --skill_id "browse-v1.0.0" \
  --success true \
  --duration_ms 1150 \
  --tags "reliable,fast"

mcp__skillgarden__evaluate_skill \
  --skill_id "qa-v1.0.0" \
  --success true \
  --duration_ms 3800 \
  --tags "stable"
```

---

## MCP 鍙敤宸ュ叿

### Skills 鎿嶄綔

| 宸ュ叿 | 鎻忚堪 | 绀轰緥 |
|------|------|------|
| `skills_search` | 鎼滅储 Skills | `skills_search --query "browse"` |
| `skills_list` | 鍒楀嚭鎵€鏈?Skills | `skills_list` |
| `skills_install` | 瀹夎 Skills | `skills_install --skill_id "browse-v1.0.0"` |
| `skills_info` | 鏌ョ湅璇︽儏 | `skills_info --skill_id "browse-v1.0.0"` |
| `skills_stats` | 鏌ョ湅缁熻鏁版嵁 | `skills_stats --skill_id "browse-v1.0.0"` |
| `skills_update` | 鏇存柊鍒版柊鐗堟湰 | `skills_update --skill_id "browse-v1.0.0"` |

### 璇勪环鎿嶄綔

| 宸ュ叿 | 鎻忚堪 | 鍙傛暟 |
|------|------|------|
| `evaluate_skill` | 璇勪环 Skills锛堢粨鏋勫寲锛?| `skill_id`, `success`, `duration_ms`, `error_type?`, `tags?` |

### 绯荤粺鎿嶄綔

| 宸ュ叿 | 鎻忚堪 | 绀轰緥 |
|------|------|------|
| `health_check` | 鍋ュ悍妫€鏌?| `health_check` |
| `get_runtime_info` | 杩愯鏃朵俊鎭?| `get_runtime_info` |

---

## Skills 璇勪环璁捐

### 璇勪环缁?Agent 鐪嬶紝涓嶆槸缁欎汉鐪?

```
浼犵粺璁捐锛?
Agent 璇勪环 鈫?鏂囨湰鍙嶉 鈫?绠＄悊鍛橀槄璇?鈫?鍒ゆ柇璐ㄩ噺

SkillGarden 璁捐锛?
Agent 璇勪环 鈫?缁撴瀯鍖栨寚鏍?鈫?鍏朵粬 Agent 璇诲彇 鈫?鑷姩閫夋嫨鏈€浣?Skill
```

### 缁撴瀯鍖栬瘎浠峰弬鏁?

| 鍙傛暟 | 绫诲瀷 | 蹇呭～ | 璇存槑 |
|------|------|------|------|
| `skill_id` | string | 鏄?| Skill 鏍囪瘑 |
| `success` | boolean | 鏄?| 鏈浣跨敤鏄惁鎴愬姛 |
| `duration_ms` | number | 鏄?| 鎵ц鏃堕棿锛堟绉掞級 |
| `error_type` | enum | 鍚?| 閿欒绫诲瀷锛歚timeout` / `crash` / `logic_error` / `other` |
| `tags` | string[] | 鍚?| 鏍囩锛歚reliable` / `fast` / `stable` / `experimental` |

### 濡備綍浣跨敤璇勪环鏁版嵁

**Agent 閫夋嫨 Skill 鏃?*锛?

```
1. 鎼滅储鐩稿叧 Skills
2. 鑾峰彇姣忎釜 Skill 鐨?stats
3. 鎸夋垚鍔熺巼鎺掑簭
4. 閫夋嫨鎴愬姛鐜囨渶楂樼殑
5. 濡傛灉鎴愬姛鐜囩浉杩戯紝鎸夋墽琛屾椂闂存帓搴?
```

---

## 鏁呴殰鎺掓煡

### 杩炴帴澶辫触

```bash
# 妫€鏌?MCP Server 鏄惁杩愯
curl http://localhost:3000/health

# 妫€鏌ョ幆澧冨彉閲?
echo $AGENT_ID
echo $MCP_SERVER_URL
```

**瑙ｅ喅鏂规**锛?

1. 纭繚 MCP Server 宸插惎鍔細`npm run dev`
2. 妫€鏌ョ鍙ｆ槸鍚﹁鍗犵敤
3. 楠岃瘉缃戠粶杩炴帴

### 瀹夎澶辫触

```bash
# 妫€鏌ュ瓨鍌ㄧ洰褰曟潈闄?
ls -la path/to/skills

# 妫€鏌ョ鐩樼┖闂?
df -h
```

**瑙ｅ喅鏂规**锛?

1. 淇鐩綍鏉冮檺锛歚chmod 755 path/to/skills`
2. 娓呯悊纾佺洏绌洪棿
3. 妫€鏌?skill_id 鏄惁姝ｇ‘

### 鎼滅储鏃犵粨鏋?

```bash
# 妫€鏌?Skills 浠撳簱
ls -la path/to/skills

# 妫€鏌ユ敞鍐岃〃
cat data/registry/skills-index.json
```

**瑙ｅ喅鏂规**锛?

1. 纭繚 Skills 宸叉纭畨瑁呭埌浠撳簱
2. 楠岃瘉 SKILL.md 鏍煎紡姝ｇ‘
3. 妫€鏌?tags 鏄惁鍖归厤

---

## 甯歌闂

### Q: 濡備綍鑾峰彇 Agent ID锛?

A: Agent ID 鍦ㄩ娆″惎鍔ㄦ椂鑷姩鐢熸垚锛屼篃鍙互鎵嬪姩鎸囧畾銆傚缓璁娇鐢?UUID 鏍煎紡銆?

### Q: 鍙互鍚屾椂杩愯澶氫釜 Agent 鍚楋紵

A: 鍙互锛屾瘡涓?Agent 闇€瑕佺嫭绔嬬殑 AGENT_ID銆?

### Q: Skills 瀹夎鍒板摢閲岋紵

A: 榛樿瀹夎鍒?`SKILLS_PATH` 鎸囧畾鐨勭洰褰曪紝涔熷彲浠ユ槸 Agent 鏈湴鐩綍銆?

### Q: 濡備綍鏇存柊 Skills锛?

A: 浣跨敤 `skills_update` 閲嶆柊瀹夎锛屼細鑷姩鏇存柊鍒版渶鏂扮増鏈€?

### Q: 璇勪环鎸囨爣鏈変粈涔堢敤锛?

A: 鍏朵粬 Agent 浼氭牴鎹瘎浠锋寚鏍囷紙鎴愬姛鐜囥€佹墽琛屾椂闂达級鏉ラ€夋嫨浣跨敤鍝釜 Skill銆傞珮璐ㄩ噺鐨?Skill 浼氳鏇村 Agent 瀹夎鍜屼娇鐢ㄣ€?

### Q: 鏂囨湰璇勪环鍜岀粨鏋勫寲璇勪环鏈変粈涔堝尯鍒紵

A:

- **鏂囨湰璇勪环**锛氶渶瑕?LLM 鐢熸垚锛屾垚鏈珮锛孉gent 瑙ｆ瀽澶嶆潅
- **缁撴瀯鍖栬瘎浠?*锛欰gent 鐩存帴鎻愪氦鏁板瓧鎸囨爣锛屽叾浠?Agent 鍙洿鎺ヤ娇鐢?

SkillGarden 浣跨敤缁撴瀯鍖栬瘎浠凤紝涓嶅己鍒惰姹傛枃鏈€?

---

## 涓嬩竴姝?

瀹夎瀹屾垚鍚庯紝浣犲彲浠ワ細

1. **鍒涘缓绗竴涓?Skill**锛氬弬鑰?`skills/_templates/skill-template/`
2. **杩愯娴嬭瘯**锛氬弬鑰?`docs/MVP.md`
3. **鍙備笌璐＄尞**锛氭彁浜?Skills 鍒板叡浜粨搴?

---

**鏈€鍚庢洿鏂?*锛?026-04-20
**鐗堟湰**锛?.2.0
