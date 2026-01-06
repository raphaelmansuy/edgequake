#!/usr/bin/env python3
"""Extended Challenge Query Test Suite for EdgeQuake Search Validation

This script tests the full set of questions from specs/fix_search/questions/01-question.md
and tracks response quality across all query modes.
"""

import requests
import json
import sys
from typing import Optional

BASE_URL = "http://localhost:8080"

# Full test queries from the questions file
TEST_QUERIES = [
    {
        "id": "Q1_STLA_BYD",
        "theme": "Electrification & Autonomy",
        "query": "J'ai testé le BYD Seal U qui offre une grosse batterie LFP à un prix très bas. Concrètement, qu'est-ce que la plateforme STLA Medium du E-3008 m'apporte de plus en termes d'efficience réelle sur autoroute et de vitesse de recharge par rapport au chinois ?",
        "expected_entities": ["BYD Seal U", "E-3008", "STLA Medium", "LFP"],
        "mode": "hybrid"
    },
    {
        "id": "Q2_E208_R5",
        "theme": "Electrification & Autonomy",
        "query": "J'hésite avec la future Renault 5 ou une BYD Dolphin. La E-208 a été restylée, mais son autonomie WLTP est-elle fiable en hiver par rapport à la pompe à chaleur de la Renault ?",
        "expected_entities": ["E-208", "Renault 5", "BYD Dolphin"],
        "mode": "hybrid"
    },
    {
        "id": "Q3_ALLURE_CARE",
        "theme": "Warranty & Service",
        "query": "BYD garantit ses batteries très longtemps. J'ai vu que Peugeot a lancé Allure Care garantissant le véhicule 8 ans/160 000 km. Est-ce que cela couvre vraiment tout comme chez Kia/Hyundai ou y a-t-il des exclusions majeures ?",
        "expected_entities": ["Allure Care", "BYD", "Peugeot"],
        "mode": "hybrid"
    },
    {
        "id": "Q4_PEUGEOT_2008",
        "theme": "Product Specs",
        "query": "Quels sont les caractéristiques d'une Peugeot 2008 ?",
        "expected_entities": ["Peugeot 2008"],
        "mode": "local"
    },
    {
        "id": "Q5_ICOCKPIT_GOOGLE",
        "theme": "Technology & Infotainment",
        "query": "Je sors d'un essai du Renault Austral/Rafale et leur système OpenR Link avec Google intégré est ultra-fluide. Le nouveau i-Cockpit Panoramique de Peugeot est beau, mais est-il aussi réactif et connecté ?",
        "expected_entities": ["i-Cockpit", "OpenR Link", "Renault Austral"],
        "mode": "hybrid"
    },
    {
        "id": "Q6_408_ITOGGLE",
        "theme": "Technology & Ergonomics",
        "query": "Le design de la 408 me plaît, c'est très différent de ce que fait BYD. Mais à l'usage, l'interface des i-Toggles est-elle vraiment personnalisable ou est-ce un gadget ?",
        "expected_entities": ["Peugeot 408", "i-Toggles", "BYD"],
        "mode": "local"
    },
    {
        "id": "Q7_HYBRID_136",
        "theme": "Hybrid Motorization",
        "query": "Je ne suis pas encore sûr de passer au 100% électrique comme le veut BYD. Comment se comporte votre nouveau moteur Hybride 136 e-DCS6 ? Y a-t-il encore des à-coups ?",
        "expected_entities": ["Hybride 136", "e-DCS6", "BYD"],
        "mode": "hybrid"
    },
    {
        "id": "Q8_PHEV_CONSUMPTION",
        "theme": "Hybrid Motorization",
        "query": "Si je prends une 308 ou 408 Hybride Rechargeable, quelle est la consommation réelle une fois la batterie vide ?",
        "expected_entities": ["308 Hybrid", "408 Hybrid", "PHEV"],
        "mode": "local"
    },
    {
        "id": "Q9_BONUS_ECOLOGIQUE",
        "theme": "Economy & TCO",
        "query": "Les BYD sont moins chères à l'achat, mais elles ont perdu le bonus écologique en France. Si je configure un E-2008 ou un E-3008 Made in France, l'écart de prix final est-il compensé par le bonus et la valeur de revente ?",
        "expected_entities": ["E-2008", "E-3008", "BYD", "bonus écologique"],
        "mode": "hybrid"
    },
    {
        "id": "Q10_E3008_SCENIC",
        "theme": "Premium Positioning",
        "query": "Peugeot se veut Access Premium. Par rapport à un Renault Scénic qui est Voiture de l'année, qu'est-ce qui justifie l'écart de prix sur un E-3008 en finition GT ?",
        "expected_entities": ["E-3008", "Renault Scenic", "GT"],
        "mode": "hybrid"
    },
    {
        "id": "Q11_DRIVING_DYNAMICS",
        "theme": "Driving Pleasure",
        "query": "J'ai trouvé la BYD Atto 3 un peu molle en suspension et la direction floue. Peugeot est réputé pour son châssis. Sur des véhicules aussi lourds que les électriques actuels, avez-vous gardé le toucher de route Peugeot ?",
        "expected_entities": ["BYD Atto 3", "Peugeot", "châssis"],
        "mode": "local"
    }
]

def check_health() -> bool:
    """Check if the EdgeQuake API is running."""
    try:
        r = requests.get(f"{BASE_URL}/health", timeout=5)
        return r.status_code == 200
    except:
        return False

def query_edgequake(query: str, mode: str = "hybrid") -> dict:
    """Send a query to EdgeQuake and return the response."""
    try:
        r = requests.post(
            f"{BASE_URL}/api/v1/query",
            json={
                "query": query,
                "mode": mode,
                "top_k": 10
            },
            headers={"Content-Type": "application/json"},
            timeout=60
        )
        if r.status_code == 200:
            data = r.json()
            content = data.get("answer", "")
            sources = data.get("sources", [])
            return {
                "success": True,
                "content": content,
                "length": len(content),
                "sources_count": len(sources),
                "top_score": sources[0].get("score", 0) if sources else 0
            }
        return {"success": False, "error": f"HTTP {r.status_code}", "content": ""}
    except Exception as e:
        return {"success": False, "error": str(e), "content": ""}

def assess_quality(response: dict, expected_entities: list) -> dict:
    """Assess the quality of the response."""
    content = response.get("content", "").lower()
    
    # Check if response indicates COMPLETE lack of information (vs partial)
    # Only flag as NO_INFO if the whole response is about not having info
    no_info_phrases = [
        "ne contient pas d'information",
        "no information available",
        "cannot provide any",
        "i don't have any",
        "pas d'information disponible",
        "cannot find any relevant",
        "no relevant information"
    ]
    
    # Count substantial content vs disclaimers
    has_complete_no_info = any(phrase in content for phrase in no_info_phrases)
    
    # If we have substantial content (>500 chars) with some info, it's not NO_INFO
    # even if there are partial disclaimers
    length = response.get("length", 0)
    
    # Check for expected entities mentioned
    entities_found = []
    for entity in expected_entities:
        if entity.lower() in content:
            entities_found.append(entity)
    
    # Scoring - improved logic
    if has_complete_no_info and length < 300:
        quality = "NO_INFO"
        score = 0
    elif length < 200:
        quality = "TOO_SHORT"
        score = 20
    elif length < 500:
        quality = "PARTIAL"
        score = 50
    elif length < 1000:
        quality = "GOOD"
        score = 75
    else:
        quality = "EXCELLENT"
        score = 100
    
    # Entity bonus
    entity_score = len(entities_found) / len(expected_entities) * 30 if expected_entities else 0
    
    return {
        "quality": quality,
        "score": min(100, score + entity_score),
        "has_no_info": has_complete_no_info and length < 300,
        "entities_found": entities_found,
        "entities_expected": expected_entities
    }

def main():
    print("=" * 80)
    print("EXTENDED CHALLENGE QUERY TEST SUITE - EdgeQuake Search Validation")
    print("=" * 80)
    
    if not check_health():
        print("✗ API is not ready")
        sys.exit(1)
    print("✓ API is ready\n")
    
    results = []
    total_score = 0
    
    for i, test in enumerate(TEST_QUERIES, 1):
        print(f"\n🔍 Test {i}/{len(TEST_QUERIES)}: {test['id']}")
        print(f"   Theme: {test['theme']}")
        print(f"   Mode: {test['mode']}")
        print(f"   Query: {test['query'][:60]}...")
        
        response = query_edgequake(test["query"], test["mode"])
        quality = assess_quality(response, test["expected_entities"])
        
        result = {
            "id": test["id"],
            "theme": test["theme"],
            "mode": test["mode"],
            "length": response.get("length", 0),
            "sources": response.get("sources_count", 0),
            "quality": quality["quality"],
            "score": quality["score"],
            "entities_found": quality["entities_found"],
            "has_no_info": quality["has_no_info"]
        }
        results.append(result)
        total_score += quality["score"]
        
        # Print result
        status = "✓" if quality["score"] >= 50 else "⚠️" if quality["score"] >= 25 else "✗"
        print(f"   {status} Response: {response.get('length', 0)} chars, {response.get('sources_count', 0)} sources")
        print(f"   {status} Quality: {quality['quality']} (score: {quality['score']:.0f})")
        print(f"   {status} Entities: {quality['entities_found']} / {test['expected_entities']}")
        
        if quality["has_no_info"]:
            print(f"   ⚠️ WARNING: Response indicates 'no information'")
    
    # Summary
    avg_score = total_score / len(TEST_QUERIES)
    print("\n" + "=" * 80)
    print("SUMMARY")
    print("=" * 80)
    print(f"\nTotal tests: {len(TEST_QUERIES)}")
    print(f"Average score: {avg_score:.1f}/100")
    
    # Count by quality
    quality_counts = {}
    for r in results:
        q = r["quality"]
        quality_counts[q] = quality_counts.get(q, 0) + 1
    
    print("\nQuality distribution:")
    for quality, count in sorted(quality_counts.items()):
        pct = count / len(results) * 100
        print(f"  {quality}: {count} ({pct:.0f}%)")
    
    # Identify failures
    failures = [r for r in results if r["score"] < 50]
    if failures:
        print(f"\n⚠️ {len(failures)} tests need improvement:")
        for f in failures:
            print(f"  - {f['id']}: {f['quality']} ({f['score']:.0f})")
    
    # Save results
    with open("/tmp/extended_challenge_results.json", "w") as f:
        json.dump({
            "total_tests": len(TEST_QUERIES),
            "average_score": avg_score,
            "quality_distribution": quality_counts,
            "results": results
        }, f, indent=2)
    
    print(f"\nResults saved to /tmp/extended_challenge_results.json")

if __name__ == "__main__":
    main()
